use crate::{
    activity::{self, ActivitySlice},
    data_files,
    keyboard::{self, KeyboardRecord},
    settings,
};
use chrono::Local;
use serde::Serialize;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
};

const KEYBOARD_BUCKET_MILLIS: u64 = 60_000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalDataStatus {
    directory: String,
    retention_days: Option<u16>,
    file_count: usize,
    size_bytes: u64,
    last_write_at: Option<u64>,
    activity_records: usize,
    keyboard_records: usize,
    skipped_records: usize,
    start_at: Option<u64>,
    end_at: Option<u64>,
    health: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportResult {
    format: String,
    path: String,
    bytes: u64,
    activity_records: usize,
    keyboard_records: usize,
    skipped_records: usize,
    start_at: Option<u64>,
    end_at: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonExport<'a> {
    version: u8,
    exported_at: u64,
    activity: &'a [ActivitySlice],
    keyboard: &'a [KeyboardRecord],
}

fn record_range(
    activity: &[ActivitySlice],
    keyboard: &[KeyboardRecord],
) -> (Option<u64>, Option<u64>) {
    let activity_start = activity.iter().map(|record| record.start).min();
    let keyboard_start = keyboard.iter().map(|record| record.start).min();
    let activity_end = activity.iter().map(|record| record.end).max();
    let keyboard_end = keyboard
        .iter()
        .map(|record| record.start.saturating_add(KEYBOARD_BUCKET_MILLIS))
        .max();
    (
        activity_start.into_iter().chain(keyboard_start).min(),
        activity_end.into_iter().chain(keyboard_end).max(),
    )
}

fn read_records_from(
    root: &Path,
) -> Result<(Vec<ActivitySlice>, Vec<KeyboardRecord>, usize, u64), String> {
    let (activity, activity_skipped, activity_updated) =
        activity::read_all_records_from(root).map_err(|error| error.message)?;
    let (keyboard, keyboard_skipped, keyboard_updated) = keyboard::read_all_records_from(root)?;
    Ok((
        activity,
        keyboard,
        activity_skipped + keyboard_skipped,
        activity_updated.max(keyboard_updated),
    ))
}

fn status_from(root: &Path, retention_days: Option<u16>) -> Result<LocalDataStatus, String> {
    let files = data_files::all_record_files(root)?;
    let size_bytes = files.iter().try_fold(0u64, |total, path| {
        fs::metadata(path)
            .map(|metadata| total.saturating_add(metadata.len()))
            .map_err(|error| error.to_string())
    })?;
    let (activity, keyboard, skipped_records, last_write_at) = read_records_from(root)?;
    let (start_at, end_at) = record_range(&activity, &keyboard);
    let (health, message) = if skipped_records > 0 {
        (
            "degraded",
            format!("数据可用；已跳过 {skipped_records} 条损坏或不兼容记录"),
        )
    } else if activity.is_empty() && keyboard.is_empty() {
        ("empty", "数据目录已就绪，当前没有本地记录".to_string())
    } else {
        ("ready", "本地记录可读取并可导出".to_string())
    };
    Ok(LocalDataStatus {
        directory: root.display().to_string(),
        retention_days,
        file_count: files.len(),
        size_bytes,
        last_write_at: (last_write_at > 0).then_some(last_write_at),
        activity_records: activity.len(),
        keyboard_records: keyboard.len(),
        skipped_records,
        start_at,
        end_at,
        health,
        message,
    })
}

#[tauri::command]
pub(crate) fn get_local_data_status() -> Result<LocalDataStatus, String> {
    status_from(&data_files::data_dir()?, settings::load_data_retention()?)
}

#[tauri::command]
pub(crate) fn set_data_retention(retention_days: Option<u16>) -> Result<LocalDataStatus, String> {
    settings::save_data_retention(retention_days)?;
    data_files::cleanup_expired(retention_days)?;
    get_local_data_status()
}

pub(crate) fn apply_saved_retention() -> Result<usize, String> {
    data_files::migrate_legacy_files()?;
    data_files::cleanup_expired(settings::load_data_retention()?)
}

#[tauri::command]
pub(crate) fn open_local_data_directory() -> Result<(), String> {
    let root = data_files::data_dir()?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    #[cfg(windows)]
    {
        Command::new("explorer.exe")
            .arg(&root)
            .spawn()
            .map_err(|error| format!("无法打开数据目录：{error}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        Err("打开数据目录目前只支持 Windows".into())
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn csv_bytes(activity: &[ActivitySlice], keyboard: &[KeyboardRecord]) -> Vec<u8> {
    let mut output = String::from(
        "recordType,start,end,generation,deviceState,appId,appName,aiTool,keyStrokes\r\n",
    );
    for record in activity {
        let device_state = match record.observation.device_state {
            activity::DeviceState::Active => "active",
            activity::DeviceState::Idle => "idle",
            activity::DeviceState::Locked => "locked",
            activity::DeviceState::Unknown => "unknown",
        };
        output.push_str(&format!(
            "activity,{},{},{},{},{},{},{},\r\n",
            record.start,
            record.end,
            record.generation,
            device_state,
            csv_escape(record.observation.app_id.as_deref().unwrap_or("")),
            csv_escape(record.observation.app_name.as_deref().unwrap_or("")),
            record.observation.ai_tool
        ));
    }
    for record in keyboard {
        output.push_str(&format!(
            "keyboard,{},{},{},,,,,{}\r\n",
            record.start,
            record.start.saturating_add(KEYBOARD_BUCKET_MILLIS),
            record.generation,
            record.key_strokes
        ));
    }
    output.into_bytes()
}

fn write_export_from(root: &Path, format: &str) -> Result<ExportResult, String> {
    let (activity, keyboard, skipped_records, _) = read_records_from(root)?;
    let (start_at, end_at) = record_range(&activity, &keyboard);
    let bytes = match format {
        "json" => {
            let mut bytes = serde_json::to_vec_pretty(&JsonExport {
                version: 1,
                exported_at: data_files::unix_millis(),
                activity: &activity,
                keyboard: &keyboard,
            })
            .map_err(|error| error.to_string())?;
            bytes.push(b'\n');
            bytes
        }
        "csv" => csv_bytes(&activity, &keyboard),
        _ => return Err("导出格式只支持 JSON 或 CSV".into()),
    };
    let exports = root.join("Exports");
    fs::create_dir_all(&exports).map_err(|error| error.to_string())?;
    let stamp = Local::now().format("%Y%m%d-%H%M%S-%3f");
    let path = exports.join(format!("iTime-export-{stamp}.{format}"));
    let temp = path.with_extension(format!("{format}.tmp"));
    let mut file = File::create(&temp).map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(&temp, &path).map_err(|error| error.to_string())?;
    Ok(ExportResult {
        format: format.to_string(),
        path: path.display().to_string(),
        bytes: bytes.len() as u64,
        activity_records: activity.len(),
        keyboard_records: keyboard.len(),
        skipped_records,
        start_at,
        end_at,
    })
}

pub(crate) fn export(format: &str) -> Result<ExportResult, String> {
    write_export_from(&data_files::data_dir()?, format)
}

pub(crate) fn clear_records() -> Result<usize, String> {
    let root = data_files::data_dir()?;
    clear_records_from(&root)
}

fn clear_records_from(root: &Path) -> Result<usize, String> {
    let files = data_files::all_record_files(root)?;
    let mut removed = 0;
    let mut failures = Vec::new();
    for path in files {
        match fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(error) => failures.push(format!("{}：{error}", path.display())),
        }
    }
    if failures.is_empty() {
        Ok(removed)
    } else {
        Err(format!("部分本地数据无法删除：{}", failures.join("；")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::{model::ActivityObservation, DeviceState};
    use std::path::PathBuf;

    fn fixture_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "itime-data-management-{name}-{}-{}",
            std::process::id(),
            data_files::unix_millis()
        ))
    }

    fn write_fixture(root: &Path) {
        fs::create_dir_all(root).unwrap();
        let activity = ActivitySlice {
            version: 1,
            start: 1_000,
            end: 2_000,
            generation: 1,
            observation: ActivityObservation {
                device_state: DeviceState::Active,
                app_id: Some("code".into()),
                app_name: Some("Visual Studio Code".into()),
                ai_tool: false,
            },
        };
        fs::write(
            root.join("activity-v1.jsonl"),
            format!("{}\nbad-line\n", serde_json::to_string(&activity).unwrap()),
        )
        .unwrap();
        fs::write(
            root.join("keyboard-v1.jsonl"),
            "{\"version\":1,\"start\":60000,\"generation\":1,\"keyStrokes\":3}\n",
        )
        .unwrap();
    }

    #[test]
    fn status_reports_range_size_counts_and_degraded_recovery() {
        let root = fixture_root("status");
        write_fixture(&root);
        let status = status_from(&root, None).unwrap();
        assert_eq!(status.file_count, 2);
        assert_eq!(status.activity_records, 1);
        assert_eq!(status.keyboard_records, 1);
        assert_eq!(status.skipped_records, 1);
        assert_eq!(status.health, "degraded");
        assert_eq!(status.start_at, Some(1_000));
        assert_eq!(status.end_at, Some(120_000));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn json_and_csv_exports_can_be_read_back_with_matching_counts() {
        let root = fixture_root("export");
        write_fixture(&root);
        let json = write_export_from(&root, "json").unwrap();
        let json_value: serde_json::Value =
            serde_json::from_slice(&fs::read(&json.path).unwrap()).unwrap();
        assert_eq!(json_value["activity"].as_array().unwrap().len(), 1);
        assert_eq!(json_value["keyboard"].as_array().unwrap().len(), 1);
        let exported_duration = json_value["activity"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["end"].as_u64().unwrap() - record["start"].as_u64().unwrap())
            .sum::<u64>();
        let exported_key_strokes = json_value["keyboard"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["keyStrokes"].as_u64().unwrap())
            .sum::<u64>();
        assert_eq!(exported_duration, 1_000);
        assert_eq!(exported_key_strokes, 3);

        let csv = write_export_from(&root, "csv").unwrap();
        let csv_text = fs::read_to_string(&csv.path).unwrap();
        assert_eq!(csv_text.lines().count(), 3);
        assert!(csv_text.contains("activity,1000,2000"));
        assert!(csv_text.contains("keyboard,60000,120000"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn clear_removes_records_but_preserves_exports() {
        let root = fixture_root("clear");
        write_fixture(&root);
        let export = write_export_from(&root, "json").unwrap();
        assert_eq!(clear_records_from(&root).unwrap(), 2);
        assert!(Path::new(&export.path).is_file());
        assert!(data_files::all_record_files(&root).unwrap().is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
