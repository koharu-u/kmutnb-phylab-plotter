use crate::{
    app::ScaleSettings,
    data::LabPoint,
    stats::{linear_regression, Regression},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotScale {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub x_major: f64,
    pub y_major: f64,
    pub x_minor: f64,
    pub y_minor: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct PlotOptions {
    pub width: usize,
    pub height: usize,
    pub selected_row: Option<usize>,
    pub show_fit: bool,
    pub graph_paper_mode: bool,
    pub crosshair: Option<(f64, f64)>,
    pub unicode: bool,
}

pub fn compute_scale(points: &[LabPoint], settings: &ScaleSettings) -> PlotScale {
    if settings.manual {
        return PlotScale {
            x_min: settings.x_min,
            x_max: settings.x_max,
            y_min: settings.y_min,
            y_max: settings.y_max,
            x_major: settings.x_major,
            y_major: settings.y_major,
            x_minor: settings.x_major / 5.0,
            y_minor: settings.y_major / 5.0,
        };
    }

    let (mut x_min, mut x_max, mut y_min, mut y_max) = if points.is_empty() {
        (0.0, 10.0, 0.0, 10.0)
    } else {
        let mut x_min = points[0].x;
        let mut x_max = points[0].x;
        let mut y_min = points[0].y;
        let mut y_max = points[0].y;

        for point in points {
            x_min = x_min.min(point.x);
            x_max = x_max.max(point.x);
            y_min = y_min.min(point.y);
            y_max = y_max.max(point.y);
        }

        (x_min, x_max, y_min, y_max)
    };

    if (x_max - x_min).abs() < f64::EPSILON {
        x_min -= 1.0;
        x_max += 1.0;
    }
    if (y_max - y_min).abs() < f64::EPSILON {
        y_min -= 1.0;
        y_max += 1.0;
    }

    let x_major = nice_step((x_max - x_min) / 6.0);
    let y_major = nice_step((y_max - y_min) / 6.0);
    x_min = (x_min / x_major).floor() * x_major;
    x_max = (x_max / x_major).ceil() * x_major;
    y_min = (y_min / y_major).floor() * y_major;
    y_max = (y_max / y_major).ceil() * y_major;

    if x_min > 0.0 {
        x_min = 0.0;
    }
    if y_min > 0.0 {
        y_min = 0.0;
    }

    PlotScale {
        x_min,
        x_max,
        y_min,
        y_max,
        x_major,
        y_major,
        x_minor: x_major / 5.0,
        y_minor: y_major / 5.0,
    }
}

pub fn render_plot(points: &[LabPoint], scale: PlotScale, options: PlotOptions) -> Vec<String> {
    if options.width == 0 || options.height == 0 {
        return Vec::new();
    }

    let mut grid = vec![vec![' '; options.width]; options.height];
    draw_grid(&mut grid, scale, options);

    if options.show_fit {
        if let Some(regression) = linear_regression(points) {
            draw_fit_line(&mut grid, scale, regression, options.unicode);
        }
    }

    for point in points {
        if let Some((col, row)) = map_point(point.x, point.y, scale, options.width, options.height)
        {
            let selected = options.selected_row == Some(point.row);
            grid[row][col] = marker(selected, options.unicode);
        }
    }

    if let Some((x, y)) = options.crosshair {
        draw_crosshair(&mut grid, scale, x, y, options.unicode);
    }

    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect()
}

pub fn format_value(value: f64) -> String {
    if value.abs() >= 1000.0 || (value != 0.0 && value.abs() < 0.01) {
        format!("{value:.2e}")
    } else {
        let formatted = format!("{value:.3}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

pub fn scale_summary(scale: PlotScale) -> String {
    format!(
        "x {}..{} / major {}, y {}..{} / major {}",
        format_value(scale.x_min),
        format_value(scale.x_max),
        format_value(scale.x_major),
        format_value(scale.y_min),
        format_value(scale.y_max),
        format_value(scale.y_major)
    )
}

fn draw_grid(grid: &mut [Vec<char>], scale: PlotScale, options: PlotOptions) {
    let height = grid.len();
    let width = grid[0].len();
    let minor = if options.unicode { '·' } else { '.' };
    let major = if options.unicode { '┆' } else { ':' };
    let axis_h = if options.unicode { '━' } else { '-' };
    let axis_v = if options.unicode { '┃' } else { '|' };
    let origin = if options.unicode { '╋' } else { '+' };

    for (row, cells) in grid.iter_mut().enumerate().take(height) {
        for (col, cell) in cells.iter_mut().enumerate().take(width) {
            let (x, y) = unmap_cell(col, row, scale, width, height);
            let is_x_axis = near_step(y, 0.0, scale.y_minor / 2.0);
            let is_y_axis = near_step(x, 0.0, scale.x_minor / 2.0);
            let is_x_major = near_grid(x, scale.x_major, scale.x_minor / 2.0);
            let is_y_major = near_grid(y, scale.y_major, scale.y_minor / 2.0);
            let is_x_minor = near_grid(x, scale.x_minor, scale.x_minor / 3.0);
            let is_y_minor = near_grid(y, scale.y_minor, scale.y_minor / 3.0);

            *cell = if is_x_axis && is_y_axis {
                origin
            } else if is_x_axis {
                axis_h
            } else if is_y_axis {
                axis_v
            } else if is_x_major || is_y_major {
                major
            } else if options.graph_paper_mode && (is_x_minor || is_y_minor) {
                minor
            } else {
                ' '
            };
        }
    }

    draw_tick_labels(grid, scale);
}

fn draw_tick_labels(grid: &mut [Vec<char>], scale: PlotScale) {
    let height = grid.len();
    let width = grid[0].len();

    for x in major_values(scale.x_min, scale.x_max, scale.x_major) {
        if let Some((col, row)) = map_point(x, scale.y_min, scale, width, height) {
            let label = format_value(x);
            put_text(grid, col.saturating_sub(label.len() / 2), row, &label);
        }
    }

    for y in major_values(scale.y_min, scale.y_max, scale.y_major) {
        if let Some((col, row)) = map_point(scale.x_min, y, scale, width, height) {
            let label = format_value(y);
            put_text(grid, col, row, &label);
        }
    }
}

fn draw_fit_line(grid: &mut [Vec<char>], scale: PlotScale, regression: Regression, unicode: bool) {
    let height = grid.len();
    let width = grid[0].len();
    let ch = if unicode { '╱' } else { '/' };

    for col in 0..width {
        let x = scale.x_min
            + (col as f64 / width.saturating_sub(1).max(1) as f64) * (scale.x_max - scale.x_min);
        let y = regression.slope * x + regression.intercept;
        if let Some((mapped_col, row)) = map_point(x, y, scale, width, height) {
            grid[row][mapped_col] = ch;
        }
    }
}

fn draw_crosshair(grid: &mut [Vec<char>], scale: PlotScale, x: f64, y: f64, unicode: bool) {
    let height = grid.len();
    let width = grid[0].len();
    let Some((col, row)) = map_point(x, y, scale, width, height) else {
        return;
    };

    let h = if unicode { '╌' } else { '-' };
    let v = if unicode { '╎' } else { '|' };
    for cell in grid[row].iter_mut().take(width) {
        if *cell == ' ' {
            *cell = h;
        }
    }
    for cells in grid.iter_mut().take(height) {
        if cells[col] == ' ' {
            cells[col] = v;
        }
    }
    grid[row][col] = if unicode { '╳' } else { 'X' };
}

fn map_point(
    x: f64,
    y: f64,
    scale: PlotScale,
    width: usize,
    height: usize,
) -> Option<(usize, usize)> {
    if x < scale.x_min || x > scale.x_max || y < scale.y_min || y > scale.y_max {
        return None;
    }

    let x_range = scale.x_max - scale.x_min;
    let y_range = scale.y_max - scale.y_min;
    if x_range <= 0.0 || y_range <= 0.0 {
        return None;
    }

    let col = ((x - scale.x_min) / x_range * width.saturating_sub(1) as f64).round() as usize;
    let row = ((scale.y_max - y) / y_range * height.saturating_sub(1) as f64).round() as usize;
    Some((col.min(width - 1), row.min(height - 1)))
}

fn unmap_cell(col: usize, row: usize, scale: PlotScale, width: usize, height: usize) -> (f64, f64) {
    let x = scale.x_min
        + (col as f64 / width.saturating_sub(1).max(1) as f64) * (scale.x_max - scale.x_min);
    let y = scale.y_max
        - (row as f64 / height.saturating_sub(1).max(1) as f64) * (scale.y_max - scale.y_min);
    (x, y)
}

fn near_grid(value: f64, step: f64, tolerance: f64) -> bool {
    if step <= 0.0 {
        return false;
    }

    let nearest = (value / step).round() * step;
    (value - nearest).abs() <= tolerance
}

fn near_step(value: f64, target: f64, tolerance: f64) -> bool {
    (value - target).abs() <= tolerance
}

fn nice_step(raw: f64) -> f64 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }

    let exponent = raw.log10().floor();
    let base = 10f64.powf(exponent);
    let fraction = raw / base;
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice * base
}

fn major_values(min: f64, max: f64, step: f64) -> Vec<f64> {
    if step <= 0.0 {
        return Vec::new();
    }

    let mut values = Vec::new();
    let mut value = (min / step).ceil() * step;
    let mut guard = 0;
    while value <= max + step * 0.001 && guard < 100 {
        values.push(value);
        value += step;
        guard += 1;
    }
    values
}

fn put_text(grid: &mut [Vec<char>], col: usize, row: usize, text: &str) {
    if row >= grid.len() {
        return;
    }

    let width = grid[row].len();
    for (offset, ch) in text.chars().enumerate() {
        let idx = col + offset;
        if idx < width {
            grid[row][idx] = ch;
        }
    }
}

fn marker(selected: bool, unicode: bool) -> char {
    match (selected, unicode) {
        (true, true) => '◆',
        (false, true) => '●',
        (true, false) => 'X',
        (false, false) => 'o',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_scale_includes_origin_when_data_is_positive() {
        let points = vec![LabPoint {
            row: 0,
            x: 2.0,
            y: 4.0,
        }];
        let scale = compute_scale(&points, &ScaleSettings::default());

        assert_eq!(scale.x_min, 0.0);
        assert_eq!(scale.y_min, 0.0);
        assert!(scale.x_max >= 2.0);
        assert!(scale.y_max >= 4.0);
    }
}
