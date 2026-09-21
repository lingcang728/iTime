mod pipeline;
#[cfg(windows)]
mod windows;
#[cfg(all(test, windows))]
mod windows_tests;

use std::path::PathBuf;

pub use pipeline::{extract_and_cache, try_cache_hit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSource {
    Cache,
    ShellItem,
    ShGetFileInfo,
    ExtractIcon,
    Shortcut,
    Fallback,
}

impl IconSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cache => "cache",
            Self::ShellItem => "shell_item",
            Self::ShGetFileInfo => "sh_get_file_info",
            Self::ExtractIcon => "extract_icon",
            Self::Shortcut => "shortcut",
            Self::Fallback => "fallback",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtractedIcon {
    pub png_path: PathBuf,
    pub source: IconSource,
    pub width: u32,
    pub height: u32,
}

/// Internal extraction request. `executable_path` is populated only from
/// collector-registered `path_hints` (foreground processes the collector
/// actually observed); it is never deserialized from IPC input.
#[derive(Debug, Clone)]
pub struct ExtractRequest {
    pub app_identity: String,
    pub executable_path: Option<String>,
    pub size: u32,
}

#[derive(Debug)]
pub enum ExtractError {
    NotFound(String),
    Api(String),
    Io(String),
    Encode(String),
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(message)
            | Self::Api(message)
            | Self::Io(message)
            | Self::Encode(message) => write!(formatter, "{message}"),
        }
    }
}
