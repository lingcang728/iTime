//! Windows local application icon resolution, disk cache, and async queue.
//!
//! Resolution order (when not served from cache):
//! 1. Exact desktop / Start-menu shortcut targeting the executable
//! 2. Package identity (AUMID / package asset)
//! 3. Embedded executable icon
//! 4. Non-generic Windows Shell icon
//! 5. Logical-name shortcut
//! 6. Explicit failure → frontend shows designed fallback

mod cache;
pub mod commands;
mod extract;
mod identity;
mod known_apps;
mod queue;
mod request;

pub use queue::IconService;

pub const ICON_RESOLVER_VERSION: u32 = 3;
pub const DEFAULT_ICON_SIZE: u32 = 64;
