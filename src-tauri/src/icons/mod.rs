//! Windows local application icon resolution, disk cache, and async queue.
//!
//! Resolution order (when not served from cache):
//! 1. Exact desktop / Start-menu shortcut targeting the executable
//! 2. Embedded executable icon (path from collector-registered hints only)
//! 3. Non-generic Windows Shell icon
//! 4. Logical-name shortcut
//! 5. Explicit failure → frontend shows designed fallback
//!
//! Security boundary: the `resolve_app_icon` IPC accepts only a logical
//! `appIdentity` + size. Real paths come solely from `path_hints` registered by
//! the activity collector, and UNC / device-namespace paths are rejected before
//! any filesystem or Shell probe so icon resolution can never trigger SMB
//! authentication or act as a file/PID/package existence oracle.

mod cache;
pub mod commands;
mod extract;
mod identity;
mod known_apps;
mod queue;
mod request;

pub(crate) use cache::purge_cache;
pub(crate) use identity::is_safe_local_path;
pub use queue::IconService;

/// Bumped whenever resolver semantics change; embedded in cache file names so
/// entries written under older rules (e.g. caller-supplied path identities)
/// are never reused.
pub const ICON_RESOLVER_VERSION: u32 = 4;
pub const DEFAULT_ICON_SIZE: u32 = 64;
