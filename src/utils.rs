use anyhow::{Ok, Result};
use console::Style;
// use std::fmt::Write as _;
use std::io::Write as _;

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
