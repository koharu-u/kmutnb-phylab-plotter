use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app::{App, Focus, Mode},
    file_io,
};

pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return false;
    }

    match app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::EditCell | Mode::RenameColumn | Mode::OpenFile => handle_text_input(app, key),
        Mode::Scale => handle_scale_input(app, key),
        Mode::Help => handle_help(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => return false,
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Char('t') => app.focus = Focus::Table,
        KeyCode::Char('g') => {
            app.focus = Focus::Graph;
            app.reset_crosshair_to_data();
        }
        KeyCode::Char('G') => {
            app.graph_paper_mode = !app.graph_paper_mode;
            app.set_status(if app.graph_paper_mode {
                "Graph paper mode enabled"
            } else {
                "Graph paper mode disabled"
            });
        }
        KeyCode::Char('f') => {
            app.show_fit = !app.show_fit;
            app.set_status(if app.show_fit {
                "Best-fit line enabled"
            } else {
                "Best-fit line disabled"
            });
        }
        KeyCode::Char('c') => {
            app.crosshair_enabled = !app.crosshair_enabled;
            app.focus = Focus::Graph;
            app.reset_crosshair_to_data();
            app.set_status(if app.crosshair_enabled {
                "Crosshair enabled"
            } else {
                "Crosshair disabled"
            });
        }
        KeyCode::Char('S') => app.begin_scale_editor(),
        KeyCode::Char('u') => app.use_auto_scale(),
        KeyCode::Char('a') => app.add_row(),
        KeyCode::Char('A') => app.add_column(),
        KeyCode::Char('d') => app.delete_selected_row(),
        KeyCode::Char('i') | KeyCode::Enter => app.begin_edit_cell(),
        KeyCode::Char('r') => app.begin_rename_column(),
        KeyCode::Char('s') => save(app),
        KeyCode::Char('o') => app.begin_open_file(),
        KeyCode::Char('h') | KeyCode::Left => move_left(app),
        KeyCode::Char('j') | KeyCode::Down => move_down(app),
        KeyCode::Char('k') | KeyCode::Up => move_up(app),
        KeyCode::Char('l') | KeyCode::Right => move_right(app),
        KeyCode::Esc => app.set_status("Ready"),
        _ => {}
    }

    true
}

fn handle_text_input(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.edit_buffer.clear();
            app.set_status("Cancelled");
        }
        KeyCode::Enter => commit_text_input(app),
        KeyCode::Backspace => {
            app.edit_buffer.pop();
        }
        KeyCode::Delete => {
            app.edit_buffer.clear();
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.edit_buffer.push(ch);
        }
        _ => {}
    }

    true
}

fn commit_text_input(app: &mut App) {
    match app.mode {
        Mode::EditCell => {
            let value = app.edit_buffer.clone();
            if app
                .data
                .edit_cell(app.selected_row, app.selected_col, value)
            {
                app.mode = Mode::Normal;
                app.edit_buffer.clear();
                app.set_status("Cell updated");
                app.reset_crosshair_to_data();
            } else {
                app.set_status("Could not edit cell");
            }
        }
        Mode::RenameColumn => {
            let name = app.edit_buffer.clone();
            if app.data.rename_column(app.selected_col, name) {
                app.mode = Mode::Normal;
                app.edit_buffer.clear();
                app.set_status("Column renamed");
            } else {
                app.set_status("Column name cannot be blank");
            }
        }
        Mode::OpenFile => {
            let path = PathBuf::from(app.edit_buffer.trim());
            if path.as_os_str().is_empty() {
                app.set_status("Enter a CSV path to open");
                return;
            }

            match file_io::load_csv(&path) {
                Ok(data) => {
                    app.data = data;
                    app.file_path = Some(path);
                    app.selected_row = 0;
                    app.selected_col = 0;
                    app.mode = Mode::Normal;
                    app.edit_buffer.clear();
                    app.reset_crosshair_to_data();
                    app.set_status("Loaded CSV");
                }
                Err(err) => app.set_status(format!("Load failed: {err}")),
            }
        }
        _ => {}
    }
}

fn handle_scale_input(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.scale_editor = None;
            app.mode = Mode::Normal;
            app.set_status("Cancelled scale edit");
        }
        KeyCode::Enter => app.apply_scale_editor(),
        KeyCode::Char('u') => app.use_auto_scale(),
        KeyCode::Tab | KeyCode::Down | KeyCode::Char('j') => move_scale_field(app, 1),
        KeyCode::BackTab | KeyCode::Up | KeyCode::Char('k') => move_scale_field(app, -1),
        KeyCode::Backspace => {
            if let Some(editor) = &mut app.scale_editor {
                editor.buffers[editor.selected].pop();
            }
        }
        KeyCode::Delete => {
            if let Some(editor) = &mut app.scale_editor {
                editor.buffers[editor.selected].clear();
            }
        }
        KeyCode::Char(ch) => {
            if let Some(editor) = &mut app.scale_editor {
                if !key.modifiers.contains(KeyModifiers::CONTROL) && is_scale_char(ch) {
                    editor.buffers[editor.selected].push(ch);
                }
            }
        }
        _ => {}
    }

    true
}

fn handle_help(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.mode = Mode::Normal,
        _ => {}
    }

    true
}

fn save(app: &mut App) {
    let path = app
        .file_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("lab_data.csv"));

    match file_io::save_csv(&path, &app.data) {
        Ok(()) => {
            app.file_path = Some(path.clone());
            app.set_status(format!("Saved {}", path.display()));
        }
        Err(err) => app.set_status(format!("Save failed: {err}")),
    }
}

fn move_left(app: &mut App) {
    if app.focus == Focus::Graph && app.crosshair_enabled {
        app.move_crosshair(-1.0, 0.0);
    } else {
        app.selected_col = app.selected_col.saturating_sub(1);
    }
}

fn move_right(app: &mut App) {
    if app.focus == Focus::Graph && app.crosshair_enabled {
        app.move_crosshair(1.0, 0.0);
    } else if app.selected_col + 1 < app.data.width() {
        app.selected_col += 1;
    }
}

fn move_up(app: &mut App) {
    if app.focus == Focus::Graph && app.crosshair_enabled {
        app.move_crosshair(0.0, 1.0);
    } else {
        app.selected_row = app.selected_row.saturating_sub(1);
        app.reset_crosshair_to_data();
    }
}

fn move_down(app: &mut App) {
    if app.focus == Focus::Graph && app.crosshair_enabled {
        app.move_crosshair(0.0, -1.0);
    } else if app.selected_row + 1 < app.data.height() {
        app.selected_row += 1;
        app.reset_crosshair_to_data();
    }
}

fn move_scale_field(app: &mut App, delta: isize) {
    let Some(editor) = &mut app.scale_editor else {
        return;
    };

    let len = editor.buffers.len() as isize;
    editor.selected = (editor.selected as isize + delta).rem_euclid(len) as usize;
}

fn is_scale_char(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+' | 'e' | 'E')
}
