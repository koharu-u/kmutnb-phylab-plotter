use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::{
    app::{App, Focus, Mode},
    graph::{self, PlotOptions},
    stats::linear_regression,
};

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(1)])
        .split(size);

    let body = if app.graph_paper_mode {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(5)])
            .split(vertical[0])
    } else if vertical[0].width < 100 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(vertical[0])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(vertical[0])
    };

    if app.graph_paper_mode {
        render_graph(frame, app, body[0]);
        render_stats(frame, app, body[1]);
    } else {
        render_table(frame, app, body[0]);
        let graph_parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(7)])
            .split(body[1]);
        render_graph(frame, app, graph_parts[0]);
        render_stats(frame, app, graph_parts[1]);
    }

    render_status(frame, app, vertical[1]);

    match app.mode {
        Mode::Help => render_help(frame, size),
        Mode::EditCell | Mode::RenameColumn | Mode::OpenFile => render_text_popup(frame, app, size),
        Mode::Scale => render_scale_popup(frame, app, size),
        Mode::Normal => {}
    }
}

fn render_table(frame: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(
        app.data
            .columns
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                let label = if idx == app.selected_col && app.focus == Focus::Table {
                    format!("> {name}")
                } else {
                    name.clone()
                };
                Cell::from(label).style(Style::default().fg(Color::Cyan))
            })
            .collect::<Vec<_>>(),
    )
    .style(Style::default().add_modifier(Modifier::BOLD));

    let rows = app
        .data
        .rows
        .iter()
        .enumerate()
        .map(|(row_idx, row)| {
            let cells = (0..app.data.width())
                .map(|col_idx| {
                    let value = row.get(col_idx).map(String::as_str).unwrap_or("");
                    let invalid_xy = is_invalid_xy(app, row_idx, col_idx);
                    let selected = app.focus == Focus::Table
                        && row_idx == app.selected_row
                        && col_idx == app.selected_col;

                    let style = if selected {
                        Style::default().fg(Color::Black).bg(Color::Yellow)
                    } else if invalid_xy {
                        Style::default().fg(Color::Red)
                    } else if row_idx == app.selected_row {
                        Style::default().bg(Color::DarkGray)
                    } else {
                        Style::default()
                    };
                    Cell::from(value.to_string()).style(style)
                })
                .collect::<Vec<_>>();
            Row::new(cells)
        })
        .collect::<Vec<_>>();

    let widths = app
        .data
        .columns
        .iter()
        .map(|_| Constraint::Length(12))
        .collect::<Vec<_>>();

    let title = table_title(app);
    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(panel_border(app.focus == Focus::Table)),
        )
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn render_graph(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(graph_title(app))
        .borders(Borders::ALL)
        .border_style(panel_border(app.focus == Focus::Graph));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 8 || inner.height < 4 {
        frame.render_widget(Paragraph::new("Terminal too small for graph"), inner);
        return;
    }

    let points = app.data.points();
    let scale = graph::compute_scale(&points, &app.scale);
    let crosshair = if app.crosshair_enabled {
        Some((app.crosshair_x, app.crosshair_y))
    } else {
        None
    };
    let lines = graph::render_plot(
        &points,
        scale,
        PlotOptions {
            width: inner.width as usize,
            height: inner.height as usize,
            selected_row: Some(app.selected_row),
            show_fit: app.show_fit,
            graph_paper_mode: app.graph_paper_mode,
            crosshair,
            unicode: app.unicode,
        },
    );

    let text = lines.into_iter().map(Line::from).collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(text), inner);
}

fn render_stats(frame: &mut Frame, app: &App, area: Rect) {
    let points = app.data.points();
    let scale = graph::compute_scale(&points, &app.scale);
    let regression = linear_regression(&points);
    let selected = app
        .data
        .selected_xy(app.selected_row)
        .map(|(x, y)| {
            format!(
                "selected x={}, y={}",
                graph::format_value(x),
                graph::format_value(y)
            )
        })
        .unwrap_or_else(|| "selected x/y invalid or blank".to_string());

    let equation = regression
        .map(|reg| {
            format!(
                "y = {}x + {}, R² = {}",
                graph::format_value(reg.slope),
                graph::format_value(reg.intercept),
                graph::format_value(reg.r_squared)
            )
        })
        .unwrap_or_else(|| "Need at least two valid non-vertical points for fit".to_string());

    let crosshair = if app.crosshair_enabled {
        format!(
            "crosshair x={}, y={}",
            graph::format_value(app.crosshair_x),
            graph::format_value(app.crosshair_y)
        )
    } else {
        "crosshair off".to_string()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!("points: {}", points.len()),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw("  "),
            Span::raw(equation),
        ]),
        Line::from(selected),
        Line::from(format!(
            "{} scale: {}",
            if app.scale.manual { "manual" } else { "auto" },
            graph::scale_summary(scale)
        )),
        Line::from(crosshair),
    ];

    let block = Block::default()
        .title("stats")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let file = app
        .file_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unsaved: lab_data.csv".to_string());
    let help = match app.mode {
        Mode::Normal => "?: help | i edit | a row | A col | d delete | s save | o open | S scale | G paper | q quit",
        Mode::Scale => "Enter apply | u auto scale | Esc cancel | j/k field",
        _ => "Enter confirm | Esc cancel",
    };
    let text = format!(
        " {} [{}:{}] | {} | {} ",
        app.mode_label(),
        app.focus_label(),
        file,
        app.status,
        help
    );
    let style = Style::default().fg(Color::Black).bg(Color::White);
    frame.render_widget(Paragraph::new(text).style(style), area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(area, 76, 24);
    frame.render_widget(Clear, popup);
    let text = vec![
        Line::styled("Physics Lab Plotter Help", Style::default().add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from("Navigation: h/j/k/l move table selection or graph crosshair, t focus table, g focus graph"),
        Line::from("Data: i edit cell, a add row, A add column, d delete row, r rename column"),
        Line::from("Files: s save CSV, o open CSV, q quit"),
        Line::from("Graph: f toggle best-fit line, G graph paper mode, c toggle crosshair"),
        Line::from("Scale: S set manual scale, u return to auto scale while in scale dialog"),
        Line::from(""),
        Line::from("Invalid or blank x/y values are shown in red in the table and ignored by fit/plot."),
        Line::from("Manual scale fields: x min, x max, y min, y max, x major division, y major division."),
        Line::from(""),
        Line::from("Press Esc, ?, or q to close this help screen."),
    ];
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().title("help").borders(Borders::ALL))
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn render_text_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(area, 70, 5);
    frame.render_widget(Clear, popup);
    let title = match app.mode {
        Mode::EditCell => "edit cell",
        Mode::RenameColumn => "rename column",
        Mode::OpenFile => "open CSV",
        _ => "",
    };
    let prompt = match app.mode {
        Mode::OpenFile => "Path",
        Mode::RenameColumn => "Column",
        _ => "Value",
    };
    let text = format!("{prompt}: {}", app.edit_buffer);
    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().title(title).borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn render_scale_popup(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(area, 72, 12);
    frame.render_widget(Clear, popup);
    let Some(editor) = &app.scale_editor else {
        return;
    };

    let labels = ["x min", "x max", "y min", "y max", "x major", "y major"];
    let lines = labels
        .iter()
        .enumerate()
        .map(|(idx, label)| {
            let selected = idx == editor.selected;
            let prefix = if selected { "> " } else { "  " };
            let style = if selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };
            Line::styled(format!("{prefix}{label:<8} {}", editor.buffers[idx]), style)
        })
        .chain([
            Line::from(""),
            Line::from("Enter applies manual scale. Press u for auto scale. Esc cancels."),
        ])
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title("scale").borders(Borders::ALL))
            .wrap(Wrap { trim: true }),
        popup,
    );
}

fn table_title(app: &App) -> String {
    format!(
        "data table [{} rows, {} columns]",
        app.data.height(),
        app.data.width()
    )
}

fn graph_title(app: &App) -> String {
    let mode = if app.graph_paper_mode {
        "graph paper"
    } else {
        "plot"
    };
    let fit = if app.show_fit { "fit on" } else { "fit off" };
    let (x_label, y_label) = app
        .data
        .xy_columns()
        .map(|(x_col, y_col)| {
            (
                app.data.columns[x_col].as_str(),
                app.data.columns[y_col].as_str(),
            )
        })
        .unwrap_or(("x", "y"));
    format!("{mode}: x-axis {x_label}, y-axis {y_label}, {fit}")
}

fn panel_border(active: bool) -> Style {
    if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(1);
    let height = height.min(area.height.saturating_sub(2)).max(1);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn is_invalid_xy(app: &App, row: usize, col: usize) -> bool {
    let Some((x_col, y_col)) = app.data.xy_columns() else {
        return false;
    };

    if col != x_col && col != y_col {
        return false;
    }

    app.data
        .cell(row, col)
        .map(|value| !value.trim().is_empty() && crate::data::parse_number(value).is_none())
        .unwrap_or(false)
}
