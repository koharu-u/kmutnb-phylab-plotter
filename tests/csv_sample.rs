use kmutnb_phylab_plotter::{file_io::load_csv_reader, stats::linear_regression};

#[test]
fn sample_csv_loads_and_regresses() {
    let data = load_csv_reader(include_bytes!("../samples/ohms_law.csv").as_slice()).unwrap();
    let points = data.points();
    let regression = linear_regression(&points).unwrap();

    assert_eq!(data.columns, vec!["x", "y", "voltage", "current"]);
    assert_eq!(points.len(), 4);
    assert!((regression.slope - 2.01).abs() < 0.05);
    assert!(regression.r_squared > 0.99);
}
