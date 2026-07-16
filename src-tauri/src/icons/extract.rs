mod pipeline;
#[cfg(windows)]
mod windows;
#[cfg(all(test, windows))]
mod windows_tests;

use super::identity::AppIdentityKind;
use std::path::PathBuf;

pub use pipeline::{extract_and_cache, try_cache_hit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSource {
    Cache,
    ShellItem,
    ShGetFileInfo,
    ExtractIcon,
    PackageAsset,
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
            Self::PackageAsset => "package_asset",
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

#[derive(Debug, Clone)]
pub struct ExtractRequest {
    pub app_identity: String,
    #[allow(dead_code)]
    pub identity_kind: AppIdentityKind,
    pub executable_path: Option<String>,
    pub aumid: Option<String>,
    pub package_full_name: Option<String>,
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
