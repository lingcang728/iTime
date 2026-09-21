//! Process-local Windows shell identity.
//!
//! Persistent App Paths and Start Menu registration belongs to the NSIS
//! installer. Portable builds must not claim either entry when they start.

use std::time::Duration;
use windows::{core::HSTRING, Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID};

const APP_USER_MODEL_ID: &str = "com.itime.desktop";

/// Set only the running process identity. This does not write the registry or
/// create/update shortcuts, so it is safe for installed and portable builds.
pub fn configure_process_identity() {
    let id = HSTRING::from(APP_USER_MODEL_ID);
    let _ = unsafe { SetCurrentProcessExplicitAppUserModelID(&id) };
}

/// 进程还没有任何窗口时的最后兜底提示。release 构建是 windows 子系统、
/// 没有控制台，stderr 不可见；数据目录不可用等致命错误需要让用户看见。
pub fn report_fatal_error(message: &str) {
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND},
        },
    };
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let caption: Vec<u16> = "iTime".encode_utf16().chain(Some(0)).collect();
    unsafe {
        let _ = MessageBoxW(
            HWND::default(),
            PCWSTR::from_raw(text.as_ptr()),
            PCWSTR::from_raw(caption.as_ptr()),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

/// 等待指定 PID 的旧进程完全退出（有超时兜底）。
///
/// 更新迁移重启时，旧进程把自身 PID 传给新实例；新实例等它退出后再初始化，
/// 否则单实例互斥会把新实例重定向给一个正在死亡的进程，更新后没有存活实例。
pub fn wait_for_process_exit(pid: u32, timeout: Duration) {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    };
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else {
            // 打不开通常意味着进程已经退出，直接继续。
            return;
        };
        let millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
        let _ = WaitForSingleObject(process, millis);
        let _ = CloseHandle(process);
    }
}
