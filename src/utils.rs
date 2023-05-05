use anyhow::{Ok, Result};
use console::Style;
// use std::fmt::Write as _;
use std::io::Write as _;

pub fn write2csv(value: serde_json::Value, path: &Option<String>) -> Result<String> {
    let mut wtr = csv::Writer::from_path(&path.clone().unwrap_or("result.csv".into()))?;

    if let Some(obj) = value.as_object() {
        wtr.write_record(obj.keys())?;
        let mut record = csv::StringRecord::new();
        for v in obj.values() {
            record.push_field(&serde_json::to_string(v)?);
        }

        wtr.write_record(record.as_byte_record())?;
    }
    wtr.flush()?;
    Ok(String::from("Ok"))
}

pub fn process_error_output(error: Result<()>) -> Result<()> {
    if let Err(e) = error {
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();

        if atty::is(atty::Stream::Stderr) {
            let s = Style::new().red();
            write!(stderr, "{}", s.apply_to(format!("{:?}", e)))?;
        } else {
            write!(stderr, "{:?}", e)?;
        }
    }

    Ok(())
}
