use chrono::{Local, NaiveDate, TimeZone};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const ACTIVITY_PREFIX: &str = "activity";
pub(crate) const KEYBOARD_PREFIX: &str = "keyboard";
const SCHEMA_SUFFIX: &str = "-v1.jsonl";
const MAX_SHARD_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn data_dir() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .ok_or_else(|| "Windows LOCALAPPDATA 路径不可用".to_string())?;
    Ok(PathBuf::from(local).join("iTime").join("Data"))
}

fn date_for(timestamp: u64) -> Result<NaiveDate, String> {
    let millis = i64::try_from(timestamp).map_err(|_| "记录时间超出支持范围".to_string())?;
    Local
        .timestamp_millis_opt(millis)
        .single()
        .map(|value| value.date_naive())
        .ok_or_else(|| "记录时间无法转换为本地日期".to_string())
}

fn shard_name(prefix: &str, date: NaiveDate, part: u32) -> String {
    if part == 1 {
        format!("{prefix}-{date}{SCHEMA_SUFFIX}")
    } else {
        format!("{prefix}-{date}-part{part}{SCHEMA_SUFFIX}")
    }
}

pub(crate) fn writable_shard(
    root: &Path,
    prefix: &str,
    timestamp: u64,
    record_bytes: usize,
) -> Result<PathBuf, String> {
    let date = date_for(timestamp)?;
    for part in 1..=9_999 {
        let candidate = root.join(shard_name(prefix, date, part));
        let current_size = fs::metadata(&candidate).map_or(0, |metadata| metadata.len());
        if current_size == 0 || current_size.saturating_add(record_bytes as u64) <= MAX_SHARD_BYTES
        {
            return Ok(candidate);
        }
    }
    Err("当天数据分片数量超出支持范围".into())
}

pub(crate) fn append_json_line(
    root: &Path,
    prefix: &str,
    timestamp: u64,
    json: &[u8],
) -> Result<PathBuf, String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let mut line = Vec::with_capacity(json.len() + 1);
    line.extend_from_slice(json);
    line.push(b'\n');
    let path = writable_shard(root, prefix, timestamp, line.len())?;
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    let existing_len = file.metadata().map_err(|error| error.to_string())?.len();
    if existing_len > 0 {
        file.seek(SeekFrom::End(-1))
            .map_err(|error| error.to_string())?;
        let mut last = [0u8; 1];
        file.read_exact(&mut last)
            .map_err(|error| error.to_string())?;
        if last[0] != b'\n' {
            // Isolate an interrupted final record so the next complete record remains readable.
            file.write_all(b"\n").map_err(|error| error.to_string())?;
        }
    }
    // Serialize before opening the file, then append the complete bounded record in one call.
    // Readers recover at newline boundaries if the process is interrupted during a write.
    file.write_all(&line).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    file.sync_data().map_err(|error| error.to_string())?;
    Ok(path)
}

fn is_record_name(name: &str, prefix: &str) -> bool {
    name == format!("{prefix}-v1.jsonl")
        || (name.starts_with(&format!("{prefix}-")) && name.ends_with(SCHEMA_SUFFIX))
}

pub(crate) fn record_files_in(root: &Path, prefix: &str) -> Result<Vec<PathBuf>, String> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(root)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| is_record_name(name, prefix))
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

pub(crate) fn all_record_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = record_files_in(root, ACTIVITY_PREFIX)?;
    files.extend(record_files_in(root, KEYBOARD_PREFIX)?);
    files.sort();
    Ok(files)
}

pub(crate) fn shard_date(path: &Path, prefix: &str) -> Option<NaiveDate> {
    let name = path.file_name()?.to_str()?;
    let remainder = name.strip_prefix(&format!("{prefix}-"))?;
    if remainder == "v1.jsonl" || remainder.len() < 10 {
        return None;
    }
    NaiveDate::parse_from_str(&remainder[..10], "%Y-%m-%d").ok()
}

pub(crate) fn cleanup_expired_in(
    root: &Path,
    retention_days: Option<u16>,
    today: NaiveDate,
) -> Result<usize, String> {
    let Some(retention_days) = retention_days else {
        return Ok(0);
    };
    let cutoff = today - chrono::Duration::days(i64::from(retention_days));
    let mut removed = 0;
    for prefix in [ACTIVITY_PREFIX, KEYBOARD_PREFIX] {
        for path in record_files_in(root, prefix)? {
            let Some(date) = shard_date(&path, prefix) else {
                // Legacy undated files are never removed automatically.
                continue;
            };
            // The current local-date shard is always active and must never be removed.
            if date >= cutoff || date == today {
                continue;
            }
            fs::remove_file(&path)
                .map_err(|error| format!("无法清理 {}：{error}", path.display()))?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub(crate) fn cleanup_expired(retention_days: Option<u16>) -> Result<usize, String> {
    cleanup_expired_in(&data_dir()?, retention_days, Local::now().date_naive())
}

pub(crate) fn migrate_legacy_file_in(root: &Path, prefix: &str) -> Result<usize, String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let legacy = root.join(format!("{prefix}-v1.jsonl"));
    let pending = root.join(format!(".{prefix}-v1.migrating"));
    if legacy.is_file() && pending.is_file() {
        return Err(format!(
            "{} 同时存在旧数据与未完成迁移，请保留文件并重试",
            root.display()
        ));
    }
    if legacy.is_file() {
        fs::rename(&legacy, &pending)
            .map_err(|error| format!("无法关闭旧版 {} 数据文件：{error}", prefix))?;
    }
    if !pending.is_file() {
        return Ok(0);
    }

    // Exact-line multiplicities make an interrupted migration resumable without
    // duplicating records already copied before the interruption.
    let mut existing = HashMap::<String, usize>::new();
    for path in record_files_in(root, prefix)? {
        for line in BufReader::new(fs::File::open(path).map_err(|error| error.to_string())?).lines()
        {
            let line = line.map_err(|error| error.to_string())?;
            *existing.entry(line).or_default() += 1;
        }
    }

    let mut seen = HashMap::<String, usize>::new();
    let mut unreadable = Vec::new();
    let mut migrated = 0;
    for line in BufReader::new(fs::File::open(&pending).map_err(|error| error.to_string())?).lines()
    {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                unreadable.push(format!("读取失败：{error}"));
                continue;
            }
        };
        let timestamp = serde_json::from_str::<serde_json::Value>(&line)
            .ok()
            .filter(|value| value.get("version").and_then(serde_json::Value::as_u64) == Some(1))
            .and_then(|value| value.get("start").and_then(serde_json::Value::as_u64));
        let Some(timestamp) = timestamp else {
            unreadable.push(line);
            continue;
        };
        let occurrence = seen.entry(line.clone()).or_default();
        *occurrence += 1;
        if *occurrence <= existing.get(&line).copied().unwrap_or(0) {
            continue;
        }
        append_json_line(root, prefix, timestamp, line.as_bytes())?;
        migrated += 1;
    }
    if !unreadable.is_empty() {
        let recovery = root.join("Recovery");
        fs::create_dir_all(&recovery).map_err(|error| error.to_string())?;
        let path = recovery.join(format!(
            "{prefix}-legacy-unreadable-{}.jsonl",
            unix_millis()
        ));
        let mut file = fs::File::create(&path).map_err(|error| error.to_string())?;
        for line in unreadable {
            file.write_all(line.as_bytes())
                .and_then(|()| file.write_all(b"\n"))
                .map_err(|error| error.to_string())?;
        }
        file.sync_all().map_err(|error| error.to_string())?;
    }
    fs::remove_file(&pending).map_err(|error| error.to_string())?;
    Ok(migrated)
}

pub(crate) fn migrate_legacy_files() -> Result<usize, String> {
    let root = data_dir()?;
    let activity = migrate_legacy_file_in(&root, ACTIVITY_PREFIX)?;
    let keyboard = migrate_legacy_file_in(&root, KEYBOARD_PREFIX)?;
    Ok(activity + keyboard)
}

pub(crate) fn modified_millis(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(system_millis)
        .unwrap_or(0)
}

pub(crate) fn unix_millis() -> u64 {
    system_millis(SystemTime::now()).unwrap_or(0)
}

fn system_millis(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "itime-data-files-{name}-{}-{}",
            std::process::id(),
            unix_millis()
        ))
    }

    #[test]
    fn shard_name_contains_local_record_date() {
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let path = writable_shard(Path::new("data"), ACTIVITY_PREFIX, timestamp, 100).unwrap();
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("activity-2026-07-27-v1.jsonl")
        );
    }

    #[test]
    fn retention_never_deletes_current_or_undated_legacy_files() {
        let root = fixture_root("retention");
        fs::create_dir_all(&root).unwrap();
        for name in [
            "activity-v1.jsonl",
            "activity-2026-04-01-v1.jsonl",
            "activity-2026-07-27-v1.jsonl",
            "keyboard-2026-04-01-v1.jsonl",
            "keyboard-2026-07-27-part2-v1.jsonl",
        ] {
            fs::write(root.join(name), b"{}\n").unwrap();
        }
        let removed = cleanup_expired_in(
            &root,
            Some(90),
            NaiveDate::from_ymd_opt(2026, 7, 27).unwrap(),
        )
        .unwrap();
        assert_eq!(removed, 2);
        assert!(root.join("activity-v1.jsonl").is_file());
        assert!(root.join("activity-2026-07-27-v1.jsonl").is_file());
        assert!(root.join("keyboard-2026-07-27-part2-v1.jsonl").is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn permanent_retention_removes_nothing() {
        let root = fixture_root("permanent");
        fs::create_dir_all(&root).unwrap();
        let old = root.join("activity-2020-01-01-v1.jsonl");
        fs::write(&old, b"{}\n").unwrap();
        assert_eq!(
            cleanup_expired_in(&root, None, NaiveDate::from_ymd_opt(2026, 7, 27).unwrap()).unwrap(),
            0
        );
        assert!(old.is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn append_isolates_an_interrupted_tail_before_the_next_record() {
        let root = fixture_root("tail-recovery");
        fs::create_dir_all(&root).unwrap();
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let path = writable_shard(&root, ACTIVITY_PREFIX, timestamp, 20).unwrap();
        fs::write(&path, b"{\"interrupted\":").unwrap();
        append_json_line(&root, ACTIVITY_PREFIX, timestamp, br#"{"version":1}"#).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text, "{\"interrupted\":\n{\"version\":1}\n");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn capacity_rotation_uses_a_closed_part_and_keeps_today_writable() {
        let root = fixture_root("capacity");
        fs::create_dir_all(&root).unwrap();
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let first = root.join("activity-2026-07-27-v1.jsonl");
        fs::File::create(&first)
            .unwrap()
            .set_len(MAX_SHARD_BYTES)
            .unwrap();
        let next = writable_shard(&root, ACTIVITY_PREFIX, timestamp, 20).unwrap();
        assert_eq!(
            next.file_name().and_then(|name| name.to_str()),
            Some("activity-2026-07-27-part2-v1.jsonl")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_migration_preserves_duplicates_and_quarantines_bad_lines() {
        let root = fixture_root("legacy");
        fs::create_dir_all(&root).unwrap();
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let line = format!(r#"{{"version":1,"start":{timestamp},"keyStrokes":2}}"#);
        fs::write(
            root.join("keyboard-v1.jsonl"),
            format!("{line}\n{line}\nnot-json\n"),
        )
        .unwrap();
        assert_eq!(migrate_legacy_file_in(&root, KEYBOARD_PREFIX).unwrap(), 2);
        assert!(!root.join("keyboard-v1.jsonl").exists());
        let shard = root.join("keyboard-2026-07-27-v1.jsonl");
        assert_eq!(fs::read_to_string(shard).unwrap().lines().count(), 2);
        assert_eq!(fs::read_dir(root.join("Recovery")).unwrap().count(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn interrupted_legacy_migration_skips_already_copied_occurrences() {
        let root = fixture_root("legacy-resume");
        fs::create_dir_all(&root).unwrap();
        let timestamp = Local
            .with_ymd_and_hms(2026, 7, 27, 13, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64;
        let line = format!(r#"{{"version":1,"start":{timestamp},"keyStrokes":2}}"#);
        fs::write(
            root.join(".keyboard-v1.migrating"),
            format!("{line}\n{line}\n"),
        )
        .unwrap();
        fs::write(
            root.join("keyboard-2026-07-27-v1.jsonl"),
            format!("{line}\n"),
        )
        .unwrap();
        assert_eq!(migrate_legacy_file_in(&root, KEYBOARD_PREFIX).unwrap(), 1);
        assert_eq!(
            fs::read_to_string(root.join("keyboard-2026-07-27-v1.jsonl"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        let _ = fs::remove_dir_all(root);
    }
}
