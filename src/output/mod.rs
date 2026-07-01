pub mod json;
pub mod text;

use serde::Serialize;

use crate::error::AppResult;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OutputMode {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy)]
pub struct Output {
    mode: OutputMode,
}

impl Output {
    /// Create an output handle in JSON or text mode.
    pub fn new(json: bool) -> Self {
        let mode = if json {
            OutputMode::Json
        } else {
            OutputMode::Text
        };
        Self { mode }
    }

    /// The current output mode.
    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    /// Print `text_line` in text mode or `json_value` in JSON mode.
    pub fn emit<T: Serialize>(&self, text_line: &str, json_value: &T) -> AppResult<()> {
        match self.mode {
            OutputMode::Text => text::print_line(text_line),
            OutputMode::Json => json::print(json_value),
        }
    }
}
