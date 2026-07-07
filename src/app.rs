use std::path::PathBuf;

use crate::{data::DataSet, graph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Table,
    Graph,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    EditCell,
    RenameColumn,
    OpenFile,
    Scale,
    Help,
}

#[derive(Debug, Clone)]
pub struct ScaleSettings {
    pub manual: bool,
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub x_major: f64,
    pub y_major: f64,
}

impl Default for ScaleSettings {
    fn default() -> Self {
        Self {
            manual: false,
            x_min: 0.0,
            x_max: 10.0,
            y_min: 0.0,
            y_max: 10.0,
            x_major: 1.0,
            y_major: 1.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScaleEditor {
    pub selected: usize,
    pub buffers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct App {
    pub data: DataSet,
    pub selected_row: usize,
    pub selected_col: usize,
    pub focus: Focus,
    pub mode: Mode,
    pub edit_buffer: String,
    pub file_path: Option<PathBuf>,
    pub status: String,
    pub show_fit: bool,
    pub graph_paper_mode: bool,
    pub crosshair_enabled: bool,
    pub crosshair_x: f64,
    pub crosshair_y: f64,
    pub scale: ScaleSettings,
    pub scale_editor: Option<ScaleEditor>,
    pub unicode: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::with_data(DataSet::new(), None)
    }
}

impl App {
    pub fn with_data(data: DataSet, file_path: Option<PathBuf>) -> Self {
        let mut app = Self {
            data,
            selected_row: 0,
            selected_col: 0,
            focus: Focus::Table,
            mode: Mode::Normal,
            edit_buffer: String::new(),
            file_path,
            status: "Ready".to_string(),
            show_fit: true,
            graph_paper_mode: false,
            crosshair_enabled: false,
            crosshair_x: 0.0,
            crosshair_y: 0.0,
            scale: ScaleSettings::default(),
            scale_editor: None,
            unicode: supports_unicode(),
        };
        app.reset_crosshair_to_data();
        app
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }

    pub fn mode_label(&self) -> &'static str {
        match self.mode {
            Mode::Normal => "NORMAL",
            Mode::EditCell => "EDIT",
            Mode::RenameColumn => "RENAME",
            Mode::OpenFile => "OPEN",
            Mode::Scale => "SCALE",
            Mode::Help => "HELP",
        }
    }

    pub fn focus_label(&self) -> &'static str {
        match self.focus {
            Focus::Table => "table",
            Focus::Graph => "graph",
        }
    }

    pub fn clamp_selection(&mut self) {
        if self.selected_col >= self.data.width() {
            self.selected_col = self.data.width().saturating_sub(1);
        }

        if self.data.height() == 0 {
            self.selected_row = 0;
        } else if self.selected_row >= self.data.height() {
            self.selected_row = self.data.height() - 1;
        }
    }

    pub fn add_row(&mut self) {
        self.data.add_row();
        self.selected_row = self.data.height().saturating_sub(1);
        self.set_status("Added row");
    }

    pub fn delete_selected_row(&mut self) {
        if self.data.delete_row(self.selected_row) {
            self.clamp_selection();
            self.set_status("Deleted row");
        } else {
            self.set_status("No row to delete");
        }
    }

    pub fn add_column(&mut self) {
        let next = self.data.width() + 1;
        self.data.add_column(format!("col{next}"));
        self.selected_col = self.data.width().saturating_sub(1);
        self.set_status("Added column; press r to rename it");
    }

    pub fn begin_edit_cell(&mut self) {
        if self.data.height() == 0 {
            self.add_row();
        }

        self.edit_buffer = self
            .data
            .cell(self.selected_row, self.selected_col)
            .unwrap_or_default()
            .to_string();
        self.mode = Mode::EditCell;
    }

    pub fn begin_rename_column(&mut self) {
        self.edit_buffer = self
            .data
            .columns
            .get(self.selected_col)
            .cloned()
            .unwrap_or_default();
        self.mode = Mode::RenameColumn;
    }

    pub fn begin_open_file(&mut self) {
        self.edit_buffer = self
            .file_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        self.mode = Mode::OpenFile;
    }

    pub fn begin_scale_editor(&mut self) {
        let current = if self.scale.manual {
            self.scale.clone()
        } else {
            let points = self.data.points();
            let scale = graph::compute_scale(&points, &self.scale);
            ScaleSettings {
                manual: true,
                x_min: scale.x_min,
                x_max: scale.x_max,
                y_min: scale.y_min,
                y_max: scale.y_max,
                x_major: scale.x_major,
                y_major: scale.y_major,
            }
        };

        self.scale_editor = Some(ScaleEditor {
            selected: 0,
            buffers: vec![
                format_number(current.x_min),
                format_number(current.x_max),
                format_number(current.y_min),
                format_number(current.y_max),
                format_number(current.x_major),
                format_number(current.y_major),
            ],
        });
        self.mode = Mode::Scale;
    }

    pub fn apply_scale_editor(&mut self) {
        let Some(editor) = &self.scale_editor else {
            return;
        };

        let values = editor
            .buffers
            .iter()
            .map(|value| value.trim().parse::<f64>())
            .collect::<Result<Vec<_>, _>>();
        let Ok(values) = values else {
            self.set_status("Scale values must be numeric");
            return;
        };

        if values[0] >= values[1] || values[2] >= values[3] {
            self.set_status("Scale min values must be smaller than max values");
            return;
        }
        if values[4] <= 0.0 || values[5] <= 0.0 {
            self.set_status("Major division sizes must be positive");
            return;
        }

        self.scale = ScaleSettings {
            manual: true,
            x_min: values[0],
            x_max: values[1],
            y_min: values[2],
            y_max: values[3],
            x_major: values[4],
            y_major: values[5],
        };
        self.scale_editor = None;
        self.mode = Mode::Normal;
        self.set_status("Manual graph scale applied");
    }

    pub fn use_auto_scale(&mut self) {
        self.scale.manual = false;
        self.scale_editor = None;
        self.mode = Mode::Normal;
        self.set_status("Auto scale enabled");
    }

    pub fn reset_crosshair_to_data(&mut self) {
        if let Some((x, y)) = self.data.selected_xy(self.selected_row) {
            self.crosshair_x = x;
            self.crosshair_y = y;
            return;
        }

        let points = self.data.points();
        let scale = graph::compute_scale(&points, &self.scale);
        self.crosshair_x = (scale.x_min + scale.x_max) / 2.0;
        self.crosshair_y = (scale.y_min + scale.y_max) / 2.0;
    }

    pub fn move_crosshair(&mut self, dx_minor: f64, dy_minor: f64) {
        let points = self.data.points();
        let scale = graph::compute_scale(&points, &self.scale);
        self.crosshair_x =
            (self.crosshair_x + dx_minor * scale.x_minor).clamp(scale.x_min, scale.x_max);
        self.crosshair_y =
            (self.crosshair_y + dy_minor * scale.y_minor).clamp(scale.y_min, scale.y_max);
    }
}

fn format_number(value: f64) -> String {
    if value.abs() >= 1000.0 || (value != 0.0 && value.abs() < 0.001) {
        format!("{value:.3e}")
    } else {
        let formatted = format!("{value:.4}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn supports_unicode() -> bool {
    let term = std::env::var("TERM").unwrap_or_default();
    if term == "dumb" {
        return false;
    }

    ["LC_ALL", "LC_CTYPE", "LANG"].iter().any(|key| {
        std::env::var(key)
            .map(|value| value.to_ascii_uppercase().contains("UTF-8"))
            .unwrap_or(false)
    })
}
