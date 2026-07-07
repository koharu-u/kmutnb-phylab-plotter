#[derive(Debug, Clone, PartialEq)]
pub struct DataSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabPoint {
    pub row: usize,
    pub x: f64,
    pub y: f64,
}

impl Default for DataSet {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSet {
    pub fn new() -> Self {
        Self {
            columns: vec!["x".to_string(), "y".to_string()],
            rows: Vec::new(),
        }
    }

    pub fn with_columns(columns: Vec<String>) -> Self {
        let columns = if columns.is_empty() {
            vec!["x".to_string(), "y".to_string()]
        } else {
            columns
        };

        Self {
            columns,
            rows: Vec::new(),
        }
    }

    pub fn width(&self) -> usize {
        self.columns.len()
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    pub fn add_row(&mut self) {
        self.rows.push(vec![String::new(); self.width()]);
    }

    pub fn add_row_values(&mut self, mut values: Vec<String>) {
        values.resize(self.width(), String::new());
        self.rows.push(values);
    }

    pub fn delete_row(&mut self, row: usize) -> bool {
        if row < self.rows.len() {
            self.rows.remove(row);
            true
        } else {
            false
        }
    }

    pub fn edit_cell(&mut self, row: usize, col: usize, value: String) -> bool {
        if row >= self.rows.len() || col >= self.width() {
            return false;
        }

        self.normalize_rows();
        self.rows[row][col] = value;
        true
    }

    pub fn rename_column(&mut self, col: usize, name: String) -> bool {
        if col >= self.width() {
            return false;
        }

        let name = name.trim();
        if name.is_empty() {
            return false;
        }

        self.columns[col] = name.to_string();
        true
    }

    pub fn add_column(&mut self, name: String) {
        let base = if name.trim().is_empty() {
            "col".to_string()
        } else {
            name.trim().to_string()
        };
        let name = self.unique_column_name(&base);
        self.columns.push(name);
        for row in &mut self.rows {
            row.push(String::new());
        }
    }

    pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
        self.rows
            .get(row)
            .and_then(|row| row.get(col))
            .map(String::as_str)
    }

    pub fn parse_cell(&self, row: usize, col: usize) -> Option<f64> {
        parse_number(self.cell(row, col)?)
    }

    pub fn xy_columns(&self) -> Option<(usize, usize)> {
        let x = self
            .columns
            .iter()
            .position(|name| name.eq_ignore_ascii_case("x"))
            .unwrap_or(0);
        let y = self
            .columns
            .iter()
            .position(|name| name.eq_ignore_ascii_case("y"))
            .unwrap_or(1);

        if x < self.width() && y < self.width() && x != y {
            Some((x, y))
        } else {
            None
        }
    }

    pub fn points(&self) -> Vec<LabPoint> {
        let Some((x_col, y_col)) = self.xy_columns() else {
            return Vec::new();
        };

        self.rows
            .iter()
            .enumerate()
            .filter_map(|(row_idx, row)| {
                let x = row.get(x_col).and_then(|value| parse_number(value))?;
                let y = row.get(y_col).and_then(|value| parse_number(value))?;
                Some(LabPoint { row: row_idx, x, y })
            })
            .collect()
    }

    pub fn selected_xy(&self, row: usize) -> Option<(f64, f64)> {
        let (x_col, y_col) = self.xy_columns()?;
        Some((self.parse_cell(row, x_col)?, self.parse_cell(row, y_col)?))
    }

    pub fn normalize_rows(&mut self) {
        let width = self.width();
        for row in &mut self.rows {
            row.resize(width, String::new());
        }
    }

    fn unique_column_name(&self, base: &str) -> String {
        if !self.columns.iter().any(|name| name == base) {
            return base.to_string();
        }

        for i in 2.. {
            let candidate = format!("{base}{i}");
            if !self.columns.iter().any(|name| name == &candidate) {
                return candidate;
            }
        }

        unreachable!()
    }
}

pub fn parse_number(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    trimmed
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_edit_delete_rows() {
        let mut data = DataSet::new();
        data.add_row();
        assert_eq!(data.height(), 1);

        assert!(data.edit_cell(0, 0, "1.5".to_string()));
        assert!(data.edit_cell(0, 1, "3.0".to_string()));
        assert_eq!(data.selected_xy(0), Some((1.5, 3.0)));

        assert!(data.delete_row(0));
        assert_eq!(data.height(), 0);
        assert!(!data.delete_row(10));
    }

    #[test]
    fn invalid_xy_values_are_excluded() {
        let mut data = DataSet::new();
        data.add_row_values(vec!["1".to_string(), "2".to_string()]);
        data.add_row_values(vec!["bad".to_string(), "4".to_string()]);
        data.add_row_values(vec!["3".to_string(), String::new()]);

        let points = data.points();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].x, 1.0);
        assert_eq!(points[0].y, 2.0);
    }
}
