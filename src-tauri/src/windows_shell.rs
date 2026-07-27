//! Process-local Windows shell identity.
//!
//! Persistent App Paths and Start Menu registration belongs to the NSIS
//! installer. Portable builds must not claim either entry when they start.

use windows::{core::HSTRING, Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID};

const APP_USER_MODEL_ID: &str = "com.itime.desktop";

/// Set only the running process identity. This does not write the registry or
/// create/update shortcuts, so it is safe for installed and portable builds.
pub fn configure_process_identity() {
    let id = HSTRING::from(APP_USER_MODEL_ID);
    let _ = unsafe { SetCurrentProcessExplicitAppUserModelID(&id) };
}
