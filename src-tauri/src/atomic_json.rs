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
