use crate::error::AppResult;

/// Print a single line to stdout.
pub fn print_line(line: &str) -> AppResult<()> {
    println!("{line}");
    Ok(())
}
