use crate::data::LabPoint;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Regression {
    pub n: usize,
    pub slope: f64,
    pub intercept: f64,
    pub r_squared: f64,
}

pub fn linear_regression(points: &[LabPoint]) -> Option<Regression> {
    if points.len() < 2 {
        return None;
    }

    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|point| point.x).sum();
    let sum_y: f64 = points.iter().map(|point| point.y).sum();
    let sum_x2: f64 = points.iter().map(|point| point.x * point.x).sum();
    let sum_xy: f64 = points.iter().map(|point| point.x * point.y).sum();

    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < f64::EPSILON {
        return None;
    }

    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n;
    let mean_y = sum_y / n;
    let ss_tot: f64 = points
        .iter()
        .map(|point| {
            let delta = point.y - mean_y;
            delta * delta
        })
        .sum();
    let ss_res: f64 = points
        .iter()
        .map(|point| {
            let predicted = slope * point.x + intercept;
            let delta = point.y - predicted;
            delta * delta
        })
        .sum();

    let r_squared = if ss_tot.abs() < f64::EPSILON {
        if ss_res.abs() < f64::EPSILON {
            1.0
        } else {
            0.0
        }
    } else {
        1.0 - ss_res / ss_tot
    };

    Some(Regression {
        n: points.len(),
        slope,
        intercept,
        r_squared,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(row: usize, x: f64, y: f64) -> LabPoint {
        LabPoint { row, x, y }
    }

    #[test]
    fn computes_linear_regression() {
        let points = vec![
            point(0, 1.0, 2.0),
            point(1, 2.0, 4.0),
            point(2, 3.0, 6.0),
            point(3, 4.0, 8.0),
        ];

        let regression = linear_regression(&points).unwrap();
        assert!((regression.slope - 2.0).abs() < 1e-12);
        assert!(regression.intercept.abs() < 1e-12);
        assert!((regression.r_squared - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_vertical_line() {
        let points = vec![point(0, 1.0, 2.0), point(1, 1.0, 3.0)];
        assert_eq!(linear_regression(&points), None);
    }
}
