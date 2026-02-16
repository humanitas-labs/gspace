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
    pub fn new(json: bool) -> Self {
        let mode = if json {
            OutputMode::Json
        } else {
            OutputMode::Text
        };
        Self { mode }
    }

    pub fn mode(&self) -> OutputMode {
        self.mode
    }

    pub fn emit<T: Serialize>(&self, text_line: &str, json_value: &T) -> AppResult<()> {
        match self.mode {
            OutputMode::Text => text::print_line(text_line),
            OutputMode::Json => json::print(json_value),
        }
    }
}
