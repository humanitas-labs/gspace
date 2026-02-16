use crate::error::AppResult;

pub fn print_line(line: &str) -> AppResult<()> {
    println!("{line}");
    Ok(())
}
