use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::{
    app::ScaleSettings,
    data::LabPoint,
    stats::{linear_regression, Regression},
};

const BRAILLE_WIDTH: usize = 2;
const BRAILLE_HEIGHT: usize = 4;
const DEFAULT_MINOR_DIVISIONS: usize = 10;

pub type GraphScale = PlotScale;

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
    pub minor_divisions: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct GridStyle {
    pub graph_paper_mode: bool,
    pub use_braille: bool,
    pub show_minor_grid: bool,
    pub show_major_grid: bool,
    pub show_axes: bool,
}

impl GridStyle {
    fn from_options(options: PlotOptions) -> Self {
        Self {
            graph_paper_mode: options.graph_paper_mode,
            use_braille: options.unicode,
            show_minor_grid: options.graph_paper_mode,
            show_major_grid: true,
            show_axes: true,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Layer {
    Background = 0,
    MinorGrid = 1,
    MajorGrid = 2,
    FitLine = 3,
    Axis = 4,
    Crosshair = 5,
    Point = 6,
    SelectedPoint = 7,
    Label = 8,
}

#[derive(Debug, Clone, Copy)]
struct OutputCell {
    ch: char,
    layer: Layer,
}

impl Default for OutputCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            layer: Layer::Background,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PlotArea {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

pub struct GraphCanvas {
    width: usize,
    height: usize,
    cells: Vec<OutputCell>,
}

impl GraphCanvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![OutputCell::default(); width * height],
        }
    }

    fn set(&mut self, col: usize, row: usize, ch: char, layer: Layer) {
        if col >= self.width || row >= self.height {
            return;
        }

        let idx = row * self.width + col;
        if layer >= self.cells[idx].layer || self.cells[idx].ch == ' ' {
            self.cells[idx] = OutputCell { ch, layer };
        }
    }

    fn put_text(&mut self, col: usize, row: usize, text: &str) {
        if row >= self.height {
            return;
        }

        for (offset, ch) in text.chars().enumerate() {
            self.set(col + offset, row, ch, Layer::Label);
        }
    }

    fn into_rows(self) -> Vec<Vec<OutputCell>> {
        self.cells
            .chunks(self.width)
            .map(|row| row.to_vec())
            .collect()
    }
}

pub struct VirtualCanvas {
    pub width_px: usize,
    pub height_px: usize,
    pixels: Vec<Layer>,
}

impl VirtualCanvas {
    pub fn new(width_px: usize, height_px: usize) -> Self {
        Self {
            width_px,
            height_px,
            pixels: vec![Layer::Background; width_px * height_px],
        }
    }

    fn set(&mut self, x: isize, y: isize, layer: Layer) {
        if x < 0 || y < 0 {
            return;
        }

        let x = x as usize;
        let y = y as usize;
        if x >= self.width_px || y >= self.height_px {
            return;
        }

        let idx = y * self.width_px + x;
        if layer >= self.pixels[idx] {
            self.pixels[idx] = layer;
        }
    }

    fn get(&self, x: usize, y: usize) -> Layer {
        if x >= self.width_px || y >= self.height_px {
            return Layer::Background;
        }

        self.pixels[y * self.width_px + x]
    }

    fn draw_vertical_line(&mut self, x: usize, layer: Layer) {
        for y in 0..self.height_px {
            self.set(x as isize, y as isize, layer);
        }
    }

    fn draw_horizontal_line(&mut self, y: usize, layer: Layer) {
        for x in 0..self.width_px {
            self.set(x as isize, y as isize, layer);
        }
    }

    fn draw_point(&mut self, x: usize, y: usize, radius: isize, layer: Layer) {
        let x = x as isize;
        let y = y as isize;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius + 1 {
                    self.set(x + dx, y + dy, layer);
                }
            }
        }
    }

    fn draw_line(&mut self, from: (usize, usize), to: (usize, usize), layer: Layer) {
        let (mut x0, mut y0) = (from.0 as isize, from.1 as isize);
        let (x1, y1) = (to.0 as isize, to.1 as isize);
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.set(x0, y0, layer);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

pub struct PlotRenderer {
    points: Vec<LabPoint>,
    scale: PlotScale,
    options: PlotOptions,
    style: GridStyle,
    area: PlotArea,
    canvas: VirtualCanvas,
}

impl PlotRenderer {
    pub fn new(points: &[LabPoint], scale: PlotScale, options: PlotOptions) -> Self {
        let style = GridStyle::from_options(options);
        let area = plot_area(options.width, options.height, scale);
        let canvas = VirtualCanvas::new(area.width * BRAILLE_WIDTH, area.height * BRAILLE_HEIGHT);

        Self {
            points: points.to_vec(),
            scale,
            options,
            style,
            area,
            canvas,
        }
    }

    fn render(mut self) -> Vec<Vec<OutputCell>> {
        if self.style.use_braille {
            self.draw_minor_grid();
            self.draw_major_grid();
            self.draw_axes();
            self.draw_fit_line();
            self.draw_crosshair();
            self.draw_points();
            self.draw_selected_point();
            self.render_to_braille()
        } else {
            self.render_clean_fallback()
        }
    }

    pub fn draw_minor_grid(&mut self) {
        if !self.style.graph_paper_mode || !self.style.show_minor_grid {
            return;
        }

        for x in grid_values(self.scale.x_min, self.scale.x_max, self.scale.x_minor) {
            if is_major_value(x, self.scale.x_major) {
                continue;
            }
            if let Some(px) = self.x_to_px(x) {
                self.canvas.draw_vertical_line(px, Layer::MinorGrid);
            }
        }

        for y in grid_values(self.scale.y_min, self.scale.y_max, self.scale.y_minor) {
            if is_major_value(y, self.scale.y_major) {
                continue;
            }
            if let Some(py) = self.y_to_px(y) {
                self.canvas.draw_horizontal_line(py, Layer::MinorGrid);
            }
        }
    }

    pub fn draw_major_grid(&mut self) {
        if !self.style.show_major_grid {
            return;
        }

        for x in major_values(self.scale.x_min, self.scale.x_max, self.scale.x_major) {
            if let Some(px) = self.x_to_px(x) {
                self.canvas.draw_vertical_line(px, Layer::MajorGrid);
            }
        }

        for y in major_values(self.scale.y_min, self.scale.y_max, self.scale.y_major) {
            if let Some(py) = self.y_to_px(y) {
                self.canvas.draw_horizontal_line(py, Layer::MajorGrid);
            }
        }
    }

    pub fn draw_axes(&mut self) {
        if !self.style.show_axes {
            return;
        }

        let axis_x = if self.scale.x_min <= 0.0 && self.scale.x_max >= 0.0 {
            self.x_to_px(0.0).unwrap_or(0)
        } else {
            0
        };
        let axis_y = if self.scale.y_min <= 0.0 && self.scale.y_max >= 0.0 {
            self.y_to_px(0.0)
                .unwrap_or_else(|| self.canvas.height_px.saturating_sub(1))
        } else {
            self.canvas.height_px.saturating_sub(1)
        };

        self.canvas.draw_vertical_line(axis_x, Layer::Axis);
        self.canvas.draw_horizontal_line(axis_y, Layer::Axis);
    }

    pub fn draw_fit_line(&mut self) {
        if !self.options.show_fit {
            return;
        }

        let Some(regression) = linear_regression(&self.points) else {
            return;
        };
        draw_fit_line_on_canvas(&mut self.canvas, self.scale, regression);
    }

    fn draw_crosshair(&mut self) {
        let Some((x, y)) = self.options.crosshair else {
            return;
        };
        let Some(px) = self.x_to_px(x) else {
            return;
        };
        let Some(py) = self.y_to_px(y) else {
            return;
        };

        self.canvas.draw_vertical_line(px, Layer::Crosshair);
        self.canvas.draw_horizontal_line(py, Layer::Crosshair);
    }

    pub fn draw_points(&mut self) {
        for point in &self.points {
            if self.options.selected_row == Some(point.row) {
                continue;
            }
            if let Some((x, y)) = self.map_point_to_px(point.x, point.y) {
                self.canvas.draw_point(x, y, 1, Layer::Point);
            }
        }
    }

    pub fn draw_selected_point(&mut self) {
        let Some(row) = self.options.selected_row else {
            return;
        };

        for point in &self.points {
            if point.row == row {
                if let Some((x, y)) = self.map_point_to_px(point.x, point.y) {
                    self.canvas.draw_point(x, y, 2, Layer::SelectedPoint);
                }
                break;
            }
        }
    }

    fn render_to_braille(&self) -> Vec<Vec<OutputCell>> {
        let mut graph = GraphCanvas::new(self.options.width, self.options.height);
        let rows = self.area.height;
        let cols = self.area.width;

        for row in 0..rows {
            for col in 0..cols {
                let mut bits = 0u32;
                let mut layer = Layer::Background;

                for dy in 0..BRAILLE_HEIGHT {
                    for dx in 0..BRAILLE_WIDTH {
                        let px = col * BRAILLE_WIDTH + dx;
                        let py = row * BRAILLE_HEIGHT + dy;
                        let dot_layer = self.canvas.get(px, py);
                        if dot_layer != Layer::Background {
                            bits |= braille_bit(dx, dy);
                            layer = layer.max(dot_layer);
                        }
                    }
                }

                if bits != 0 {
                    let ch = char::from_u32(0x2800 + bits).unwrap_or(' ');
                    graph.set(self.area.x + col, self.area.y + row, ch, layer);
                }
            }
        }

        self.draw_tick_labels(&mut graph);
        graph.into_rows()
    }

    fn render_clean_fallback(&self) -> Vec<Vec<OutputCell>> {
        let mut graph = GraphCanvas::new(self.options.width, self.options.height);

        for x in major_values(self.scale.x_min, self.scale.x_max, self.scale.x_major) {
            if let Some(col) = self.x_to_cell(x) {
                for row in self.area.y..self.area.y + self.area.height {
                    graph.set(col, row, '|', Layer::MajorGrid);
                }
            }
        }

        for y in major_values(self.scale.y_min, self.scale.y_max, self.scale.y_major) {
            if let Some(row) = self.y_to_cell(y) {
                for col in self.area.x..self.area.x + self.area.width {
                    graph.set(col, row, '-', Layer::MajorGrid);
                }
            }
        }

        let axis_col = if self.scale.x_min <= 0.0 && self.scale.x_max >= 0.0 {
            self.x_to_cell(0.0).unwrap_or(self.area.x)
        } else {
            self.area.x
        };
        let axis_row = if self.scale.y_min <= 0.0 && self.scale.y_max >= 0.0 {
            self.y_to_cell(0.0)
                .unwrap_or_else(|| self.area.y + self.area.height.saturating_sub(1))
        } else {
            self.area.y + self.area.height.saturating_sub(1)
        };
        for row in self.area.y..self.area.y + self.area.height {
            graph.set(axis_col, row, '|', Layer::Axis);
        }
        for col in self.area.x..self.area.x + self.area.width {
            graph.set(col, axis_row, '-', Layer::Axis);
        }
        graph.set(axis_col, axis_row, '+', Layer::Axis);

        if self.options.show_fit {
            if let Some(regression) = linear_regression(&self.points) {
                for col in self.area.x..self.area.x + self.area.width {
                    let x = self.cell_to_x(col);
                    let y = regression.slope * x + regression.intercept;
                    if let Some(row) = self.y_to_cell(y) {
                        graph.set(col, row, '.', Layer::FitLine);
                    }
                }
            }
        }

        if let Some((x, y)) = self.options.crosshair {
            if let (Some(col), Some(row)) = (self.x_to_cell(x), self.y_to_cell(y)) {
                for c in self.area.x..self.area.x + self.area.width {
                    graph.set(c, row, '-', Layer::Crosshair);
                }
                for r in self.area.y..self.area.y + self.area.height {
                    graph.set(col, r, '|', Layer::Crosshair);
                }
                graph.set(col, row, 'X', Layer::Crosshair);
            }
        }

        for point in &self.points {
            if self.options.selected_row == Some(point.row) {
                continue;
            }
            if let Some((col, row)) = self.map_point_to_cell(point.x, point.y) {
                graph.set(col, row, 'o', Layer::Point);
            }
        }

        if let Some(row_idx) = self.options.selected_row {
            for point in &self.points {
                if point.row == row_idx {
                    if let Some((col, row)) = self.map_point_to_cell(point.x, point.y) {
                        graph.set(col, row, 'X', Layer::SelectedPoint);
                    }
                    break;
                }
            }
        }

        self.draw_tick_labels(&mut graph);
        graph.into_rows()
    }

    fn draw_tick_labels(&self, graph: &mut GraphCanvas) {
        if self.area.height == 0 || self.area.width == 0 {
            return;
        }

        let y_values = major_values(self.scale.y_min, self.scale.y_max, self.scale.y_major);
        let mut used_y_rows = Vec::new();
        for y in y_values {
            let Some(row) = self.y_to_cell(y) else {
                continue;
            };
            if used_y_rows.iter().any(|used| row.abs_diff(*used) < 1) {
                continue;
            }
            used_y_rows.push(row);

            let label = format_value(y);
            if self.area.x == 0 {
                graph.put_text(0, row, &label);
            } else {
                let col = self.area.x.saturating_sub(label.len() + 1);
                graph.put_text(col, row, &label);
            }
        }

        let label_row = self.area.y + self.area.height;
        if label_row >= self.options.height {
            return;
        }

        let mut next_free_col = 0;
        for x in major_values(self.scale.x_min, self.scale.x_max, self.scale.x_major) {
            let Some(col) = self.x_to_cell(x) else {
                continue;
            };

            let label = format_value(x);
            let start = col.saturating_sub(label.len() / 2);
            if start < next_free_col {
                continue;
            }
            graph.put_text(start, label_row, &label);
            next_free_col = start + label.len() + 1;
        }
    }

    fn map_point_to_px(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        Some((self.x_to_px(x)?, self.y_to_px(y)?))
    }

    fn map_point_to_cell(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        Some((self.x_to_cell(x)?, self.y_to_cell(y)?))
    }

    fn x_to_px(&self, x: f64) -> Option<usize> {
        map_value(
            x,
            self.scale.x_min,
            self.scale.x_max,
            self.canvas.width_px,
            false,
        )
    }

    fn y_to_px(&self, y: f64) -> Option<usize> {
        map_value(
            y,
            self.scale.y_min,
            self.scale.y_max,
            self.canvas.height_px,
            true,
        )
    }

    fn x_to_cell(&self, x: f64) -> Option<usize> {
        map_value(
            x,
            self.scale.x_min,
            self.scale.x_max,
            self.area.width,
            false,
        )
        .map(|col| self.area.x + col)
    }

    fn y_to_cell(&self, y: f64) -> Option<usize> {
        map_value(
            y,
            self.scale.y_min,
            self.scale.y_max,
            self.area.height,
            true,
        )
        .map(|row| self.area.y + row)
    }

    fn cell_to_x(&self, col: usize) -> f64 {
        let local_col = col.saturating_sub(self.area.x);
        self.scale.x_min
            + (local_col as f64 / self.area.width.saturating_sub(1).max(1) as f64)
                * (self.scale.x_max - self.scale.x_min)
    }
}

pub fn compute_scale(points: &[LabPoint], settings: &ScaleSettings) -> PlotScale {
    let minor_divisions = settings.minor_divisions.max(1);
    if settings.manual {
        return PlotScale {
            x_min: settings.x_min,
            x_max: settings.x_max,
            y_min: settings.y_min,
            y_max: settings.y_max,
            x_major: settings.x_major,
            y_major: settings.y_major,
            x_minor: settings.x_major / minor_divisions as f64,
            y_minor: settings.y_major / minor_divisions as f64,
            minor_divisions,
        };
    }

    let (x_min, x_max, y_min, y_max) = if points.is_empty() {
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

    let (x_min, x_max, x_major) = nice_bounds(x_min, x_max);
    let (y_min, y_max, y_major) = nice_bounds(y_min, y_max);

    PlotScale {
        x_min,
        x_max,
        y_min,
        y_max,
        x_major,
        y_major,
        x_minor: x_major / DEFAULT_MINOR_DIVISIONS as f64,
        y_minor: y_major / DEFAULT_MINOR_DIVISIONS as f64,
        minor_divisions: DEFAULT_MINOR_DIVISIONS,
    }
}

pub fn render_plot_lines(
    points: &[LabPoint],
    scale: PlotScale,
    options: PlotOptions,
) -> Vec<Line<'static>> {
    let cells = render_plot_cells(points, scale, options);
    cells
        .into_iter()
        .map(|row| {
            let mut spans = Vec::new();
            let mut current_style = None;
            let mut buffer = String::new();

            for cell in row {
                let style = style_for_layer(cell.layer);
                if current_style == Some(style) {
                    buffer.push(cell.ch);
                } else {
                    if let Some(previous_style) = current_style {
                        spans.push(Span::styled(std::mem::take(&mut buffer), previous_style));
                    }
                    current_style = Some(style);
                    buffer.push(cell.ch);
                }
            }

            if let Some(style) = current_style {
                spans.push(Span::styled(buffer, style));
            }

            Line::from(spans)
        })
        .collect()
}

pub fn render_plot(points: &[LabPoint], scale: PlotScale, options: PlotOptions) -> Vec<String> {
    render_plot_cells(points, scale, options)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect())
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
        "x {}..{} / major {}, y {}..{} / major {}, minor divisions {}",
        format_value(scale.x_min),
        format_value(scale.x_max),
        format_value(scale.x_major),
        format_value(scale.y_min),
        format_value(scale.y_max),
        format_value(scale.y_major),
        scale.minor_divisions
    )
}

fn render_plot_cells(
    points: &[LabPoint],
    scale: PlotScale,
    options: PlotOptions,
) -> Vec<Vec<OutputCell>> {
    if options.width == 0 || options.height == 0 {
        return Vec::new();
    }

    PlotRenderer::new(points, scale, options).render()
}

fn draw_fit_line_on_canvas(canvas: &mut VirtualCanvas, scale: PlotScale, regression: Regression) {
    let mut previous = None;

    for px in 0..canvas.width_px {
        let x = scale.x_min
            + (px as f64 / canvas.width_px.saturating_sub(1).max(1) as f64)
                * (scale.x_max - scale.x_min);
        let y = regression.slope * x + regression.intercept;
        let Some(py) = map_value(y, scale.y_min, scale.y_max, canvas.height_px, true) else {
            previous = None;
            continue;
        };

        if let Some(from) = previous {
            canvas.draw_line(from, (px, py), Layer::FitLine);
        } else {
            canvas.set(px as isize, py as isize, Layer::FitLine);
        }
        previous = Some((px, py));
    }
}

fn plot_area(width: usize, height: usize, scale: PlotScale) -> PlotArea {
    if width < 12 || height < 5 {
        return PlotArea {
            x: 0,
            y: 0,
            width,
            height,
        };
    }

    let y_label_width = major_values(scale.y_min, scale.y_max, scale.y_major)
        .into_iter()
        .map(|value| format_value(value).len())
        .max()
        .unwrap_or(1)
        + 1;
    let left = y_label_width.clamp(4, (width / 3).max(4));
    let bottom = 1;
    let right = usize::from(width >= 24);

    if width <= left + right + 4 || height <= bottom + 3 {
        return PlotArea {
            x: 0,
            y: 0,
            width,
            height,
        };
    }

    PlotArea {
        x: left,
        y: 0,
        width: width - left - right,
        height: height - bottom,
    }
}

fn map_value(value: f64, min: f64, max: f64, size: usize, reverse: bool) -> Option<usize> {
    if value < min || value > max || size == 0 {
        return None;
    }

    let range = max - min;
    if range <= 0.0 || !range.is_finite() {
        return None;
    }

    let fraction = if reverse {
        (max - value) / range
    } else {
        (value - min) / range
    };
    let mapped = (fraction * size.saturating_sub(1) as f64).round() as usize;
    Some(mapped.min(size - 1))
}

fn braille_bit(dx: usize, dy: usize) -> u32 {
    match (dx, dy) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (0, 3) => 0x40,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        (1, 3) => 0x80,
        _ => 0,
    }
}

fn style_for_layer(layer: Layer) -> Style {
    match layer {
        Layer::Background => Style::default(),
        Layer::MinorGrid => Style::default().fg(Color::DarkGray),
        Layer::MajorGrid => Style::default().fg(Color::Gray),
        Layer::FitLine => Style::default().fg(Color::Blue).add_modifier(Modifier::DIM),
        Layer::Axis => Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        Layer::Crosshair => Style::default().fg(Color::Magenta),
        Layer::Point => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        Layer::SelectedPoint => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        Layer::Label => Style::default().fg(Color::White),
    }
}

fn nice_bounds(mut min: f64, mut max: f64) -> (f64, f64, f64) {
    if (max - min).abs() < f64::EPSILON {
        let center = min;
        let padding = if center.abs() < 1.0 {
            1.0
        } else {
            10f64.powf(center.abs().log10().floor())
        };
        min = center - padding;
        max = center + padding;
    }

    let span = max - min;
    let major = nice_step(span / 8.0);
    let mut nice_min = (min / major).floor() * major;
    let mut nice_max = (max / major).ceil() * major;
    let tolerance = major.abs() * 1e-9;

    if min - nice_min <= major * 0.05 + tolerance {
        nice_min -= major;
    }
    if nice_max - max <= major * 0.05 + tolerance {
        nice_max += major;
    }

    (clean_float(nice_min), clean_float(nice_max), major)
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

fn clean_float(value: f64) -> f64 {
    let rounded = (value * 1e12).round() / 1e12;
    if rounded.abs() < 1e-12 {
        0.0
    } else {
        rounded
    }
}

fn major_values(min: f64, max: f64, step: f64) -> Vec<f64> {
    grid_values(min, max, step)
}

fn grid_values(min: f64, max: f64, step: f64) -> Vec<f64> {
    if step <= 0.0 {
        return Vec::new();
    }

    let mut values = Vec::new();
    let mut value = (min / step).ceil() * step;
    let mut guard = 0;
    while value <= max + step * 0.001 && guard < 1000 {
        values.push(clean_float(value));
        value += step;
        guard += 1;
    }
    values
}

fn is_major_value(value: f64, major_step: f64) -> bool {
    if major_step <= 0.0 {
        return false;
    }

    let nearest = (value / major_step).round() * major_step;
    (value - nearest).abs() <= major_step.abs() * 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_scale_uses_graph_paper_major_steps_without_forcing_zero() {
        let points = vec![
            LabPoint {
                row: 0,
                x: 1.09,
                y: 0.10,
            },
            LabPoint {
                row: 1,
                x: 1.88,
                y: 0.20,
            },
        ];
        let scale = compute_scale(&points, &ScaleSettings::default());

        assert_eq!(scale.x_min, 1.0);
        assert_eq!(scale.x_max, 1.9);
        assert_eq!(scale.x_major, 0.1);
        assert_eq!(scale.y_min, 0.08);
        assert_eq!(scale.y_max, 0.22);
        assert_eq!(scale.y_major, 0.02);
        assert_eq!(scale.minor_divisions, 10);
    }

    #[test]
    fn braille_renderer_does_not_use_ascii_grid_noise() {
        let points = vec![LabPoint {
            row: 0,
            x: 1.0,
            y: 1.0,
        }];
        let scale = compute_scale(&points, &ScaleSettings::default());
        let lines = render_plot(
            &points,
            scale,
            PlotOptions {
                width: 40,
                height: 12,
                selected_row: Some(0),
                show_fit: false,
                graph_paper_mode: true,
                crosshair: None,
                unicode: true,
            },
        );
        let text = lines.join("\n");

        assert!(text
            .chars()
            .any(|ch| ('\u{2800}'..='\u{28ff}').contains(&ch)));
        assert!(!text.contains("|||||"));
        assert!(!text.contains("!!!!!"));
        assert!(!text.contains(":::::"));
    }
}
