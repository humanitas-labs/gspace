use serde::Serialize;

use crate::error::AppResult;

pub fn print<T: Serialize>(value: &T) -> AppResult<()> {
    let payload = serde_json::to_string_pretty(value)?;
    println!("{payload}");
    Ok(())
}
