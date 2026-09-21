use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub(crate) fn write(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err("JSON 文件路径无效".into());
    };
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    // Any failure leaves the half-written tmp behind; remove it here so the
    // startup sweep is only a backstop for crash-killed writes.
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer(&mut file, value).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        replace_file(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// 删除 `write` 崩溃/被强杀后遗留的 `*.tmp-<pid>` 半成品文件。
/// 仅在启动早期、尚无并发写入时调用（单实例下安全）。
pub(crate) fn cleanup_stale_temp(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_stale_temp = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.starts_with("tmp-"));
        if is_stale_temp {
            let _ = fs::remove_file(&path);
        }
    }
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let target = target
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|error| error.to_string())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    if target.exists() {
        fs::remove_file(target).map_err(|error| error.to_string())?;
    }
    fs::rename(source, target).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn atomically_replaces_existing_json() {
        let path = std::env::temp_dir().join(format!(
            "itime-atomic-json-{}-{}.json",
            std::process::id(),
            crate::provider_activity::unix_millis()
        ));
        write(&path, &json!({ "version": 1 })).unwrap();
        write(&path, &json!({ "version": 2, "ready": true })).unwrap();

        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(value, json!({ "version": 2, "ready": true }));
        let _ = std::fs::remove_file(path);
    }
}
