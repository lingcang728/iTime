//! Windows local application icon resolution, disk cache, and async queue.
//!
//! Resolution order (when not served from cache):
//! 1. Shell item / package identity (AUMID, package path)
//! 2. IShellItemImageFactory ICONONLY on executable path
//! 3. SHGetFileInfoW
//! 4. ExtractIconExW
//! 5. Start-menu shortcut shell icon
//! 6. Explicit failure → frontend shows designed fallback

mod cache;
pub mod commands;
mod extract;
mod identity;
mod known_apps;
mod queue;
mod request;

pub use queue::IconService;

pub const ICON_RESOLVER_VERSION: u32 = 1;
pub const DEFAULT_ICON_SIZE: u32 = 64;
