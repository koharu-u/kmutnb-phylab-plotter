use std::path::PathBuf;

use crate::{
    data::{DataSet, LabPoint},
    graph,
};

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
    pub minor_divisions: usize,
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
            minor_divisions: 10,
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
    pub plot_axes_swapped: bool,
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
            plot_axes_swapped: false,
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
            let points = self.plot_points();
            let scale = graph::compute_scale(&points, &self.scale);
            ScaleSettings {
                manual: true,
                x_min: scale.x_min,
                x_max: scale.x_max,
                y_min: scale.y_min,
                y_max: scale.y_max,
                x_major: scale.x_major,
                y_major: scale.y_major,
                minor_divisions: scale.minor_divisions,
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
                current.minor_divisions.to_string(),
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
            .take(6)
            .map(|value| value.trim().parse::<f64>())
            .collect::<Result<Vec<_>, _>>();
        let Ok(values) = values else {
            self.set_status("Scale values must be numeric");
            return;
        };
        let Ok(minor_divisions) = editor.buffers[6].trim().parse::<usize>() else {
            self.set_status("Minor divisions must be a whole number");
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
        if minor_divisions == 0 || minor_divisions > 50 {
            self.set_status("Minor divisions must be between 1 and 50");
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
            minor_divisions,
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

    pub fn swap_plot_axes(&mut self) {
        if self.data.xy_columns().is_none() {
            self.set_status("Cannot swap axes: need distinct x and y columns");
            return;
        }

        self.plot_axes_swapped = !self.plot_axes_swapped;
        self.reset_crosshair_to_data();

        let message = {
            let (x_label, y_label) = self.plot_axis_labels();
            format!("Swapped plot axes: x-axis {x_label}, y-axis {y_label}")
        };
        self.set_status(message);
    }

    pub fn plot_columns(&self) -> Option<(usize, usize)> {
        let (x_col, y_col) = self.data.xy_columns()?;
        if self.plot_axes_swapped {
            Some((y_col, x_col))
        } else {
            Some((x_col, y_col))
        }
    }

    pub fn plot_axis_labels(&self) -> (&str, &str) {
        self.plot_columns()
            .map(|(x_col, y_col)| {
                (
                    self.data.columns[x_col].as_str(),
                    self.data.columns[y_col].as_str(),
                )
            })
            .unwrap_or(("x", "y"))
    }

    pub fn plot_points(&self) -> Vec<LabPoint> {
        let Some((x_col, y_col)) = self.plot_columns() else {
            return Vec::new();
        };

        self.data
            .rows
            .iter()
            .enumerate()
            .filter_map(|(row_idx, row)| {
                let x = row
                    .get(x_col)
                    .and_then(|value| crate::data::parse_number(value))?;
                let y = row
                    .get(y_col)
                    .and_then(|value| crate::data::parse_number(value))?;
                Some(LabPoint { row: row_idx, x, y })
            })
            .collect()
    }

    pub fn selected_plot_xy(&self, row: usize) -> Option<(f64, f64)> {
        let (x_col, y_col) = self.plot_columns()?;
        Some((
            self.data.parse_cell(row, x_col)?,
            self.data.parse_cell(row, y_col)?,
        ))
    }

    pub fn reset_crosshair_to_data(&mut self) {
        if let Some((x, y)) = self.selected_plot_xy(self.selected_row) {
            self.crosshair_x = x;
            self.crosshair_y = y;
            return;
        }

        let points = self.plot_points();
        let scale = graph::compute_scale(&points, &self.scale);
        self.crosshair_x = (scale.x_min + scale.x_max) / 2.0;
        self.crosshair_y = (scale.y_min + scale.y_max) / 2.0;
    }

    pub fn move_crosshair(&mut self, dx_minor: f64, dy_minor: f64) {
        let points = self.plot_points();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::linear_regression;

    #[test]
    fn swapping_plot_axes_changes_mapping_without_changing_table_data() {
        let mut data = DataSet::new();
        data.add_row_values(vec!["1".to_string(), "10".to_string()]);
        data.add_row_values(vec!["2".to_string(), "30".to_string()]);
        data.add_row_values(vec!["3".to_string(), "50".to_string()]);
        let original_rows = data.rows.clone();

        let mut app = App::with_data(data, None);
        let original_points = app.plot_points();
        let original_regression = linear_regression(&original_points).unwrap();

        app.swap_plot_axes();

        let swapped_points = app.plot_points();
        let swapped_regression = linear_regression(&swapped_points).unwrap();
        let swapped_scale = graph::compute_scale(&swapped_points, &app.scale);

        assert_eq!(app.data.rows, original_rows);
        assert_eq!(app.plot_axis_labels(), ("y", "x"));
        assert_eq!(app.selected_plot_xy(0), Some((10.0, 1.0)));
        assert_eq!(
            swapped_points[0],
            LabPoint {
                row: 0,
                x: 10.0,
                y: 1.0
            }
        );
        assert!((original_regression.slope - 20.0).abs() < 1e-12);
        assert!((swapped_regression.slope - 0.05).abs() < 1e-12);
        assert!(swapped_scale.x_min <= 10.0);
        assert!(swapped_scale.x_max >= 50.0);
        assert_eq!(app.status, "Swapped plot axes: x-axis y, y-axis x");

        app.swap_plot_axes();

        assert_eq!(app.data.rows, original_rows);
        assert_eq!(app.plot_axis_labels(), ("x", "y"));
        assert_eq!(app.selected_plot_xy(0), Some((1.0, 10.0)));
    }
}
