use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use anyhow::{Context, Result};
use csv::{ReaderBuilder, WriterBuilder};

use crate::data::DataSet;

pub fn load_csv(path: &Path) -> Result<DataSet> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    load_csv_reader(file)
}

pub fn save_csv(path: &Path, data: &DataSet) -> Result<()> {
    let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
    save_csv_writer(file, data)
}

pub fn load_csv_reader<R: Read>(reader: R) -> Result<DataSet> {
    let mut reader = ReaderBuilder::new()
        .trim(csv::Trim::None)
        .flexible(true)
        .from_reader(reader);

    let headers = reader
        .headers()
        .context("reading CSV headers")?
        .iter()
        .map(str::to_string)
        .collect::<Vec<_>>();

    let mut data = DataSet::with_columns(headers);
    for record in reader.records() {
        let record = record.context("reading CSV record")?;
        data.add_row_values(record.iter().map(str::to_string).collect());
    }
    data.normalize_rows();

    Ok(data)
}

pub fn save_csv_writer<W: Write>(writer: W, data: &DataSet) -> Result<()> {
    let mut writer = WriterBuilder::new().from_writer(writer);
    writer
        .write_record(&data.columns)
        .context("writing CSV header")?;

    for row in &data.rows {
        let mut normalized = row.clone();
        normalized.resize(data.width(), String::new());
        writer
            .write_record(&normalized)
            .context("writing CSV row")?;
    }

    writer.flush().context("flushing CSV writer")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_csv_with_headers() {
        let csv = "x,y,uncertainty\n1,2.1,0.1\n2,4.0,0.2\n";
        let data = load_csv_reader(csv.as_bytes()).unwrap();

        assert_eq!(data.columns, vec!["x", "y", "uncertainty"]);
        assert_eq!(data.rows.len(), 2);
        assert_eq!(data.cell(0, 1), Some("2.1"));
    }

    #[test]
    fn saves_csv_with_headers() {
        let mut data = DataSet::with_columns(vec!["x".to_string(), "y".to_string()]);
        data.add_row_values(vec!["1".to_string(), "2.1".to_string()]);

        let mut out = Vec::new();
        save_csv_writer(&mut out, &data).unwrap();
        let written = String::from_utf8(out).unwrap();

        assert_eq!(written, "x,y\n1,2.1\n");
    }
}
