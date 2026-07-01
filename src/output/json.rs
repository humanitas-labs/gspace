use serde::Serialize;

use crate::error::AppResult;

/// Serialize a value as pretty JSON and print it to stdout.
pub fn print<T: Serialize>(value: &T) -> AppResult<()> {
    let payload = serde_json::to_string_pretty(value)?;
    println!("{payload}");
    Ok(())
}
