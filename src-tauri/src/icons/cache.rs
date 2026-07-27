use crate::icons::ICON_RESOLVER_VERSION;
use sha2::{Digest, Sha256};
use std::fs::{self, FileTimes, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_DISK_CACHE_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct CacheKeyMaterial {
    pub app_identity: String,
    pub source_path: Option<String>,
    pub source_mtime_secs: Option<u64>,
    pub size: u32,
}

pub fn icons_cache_dir() -> PathBuf {
    dirs_local_data()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("iTime")
        .join("Cache")
        .join("Icons")
}

fn dirs_local_data() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
}

pub fn ensure_cache_dir() -> std::io::Result<PathBuf> {
    let dir = icons_cache_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn cache_file_name(material: &CacheKeyMaterial) -> String {
    let mut hasher = Sha256::new();
    hasher.update(material.app_identity.as_bytes());
    hasher.update(b"|");
    if let Some(path) = &material.source_path {
        hasher.update(path.as_bytes());
    }
    hasher.update(b"|");
    if let Some(mtime) = material.source_mtime_secs {
        hasher.update(mtime.to_le_bytes());
    }
    hasher.update(b"|");
    hasher.update(material.size.to_le_bytes());
    hasher.update(b"|");
    hasher.update(ICON_RESOLVER_VERSION.to_le_bytes());
    let digest = hasher.finalize();
    format!(
        "{}_{}px_v{}.png",
        hex::encode(&digest[..16]),
        material.size,
        ICON_RESOLVER_VERSION
    )
}

pub fn cache_path_for(material: &CacheKeyMaterial) -> PathBuf {
    icons_cache_dir().join(cache_file_name(material))
}

pub fn read_cached_png(path: &Path) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    let meta = fs::metadata(path).ok()?;
    if meta.len() < 32 {
        let _ = fs::remove_file(path);
        return None;
    }
    // Quick PNG signature check
    let mut header = [0u8; 8];
    if let Ok(mut file) = fs::File::open(path) {
        if file.read_exact(&mut header).is_ok() && &header == b"\x89PNG\r\n\x1a\n" {
            touch_cache_file(path);
            if let Some(parent) = path.parent() {
                let _ = prune_cache_dir(parent, MAX_DISK_CACHE_BYTES, Some(path));
            }
            return Some(path.to_path_buf());
        }
    }
    // Corrupt cache file — drop it so next resolve regenerates
    let _ = fs::remove_file(path);
    None
}

pub fn write_cached_png(path: &Path, png_bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("png.tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&tmp)?;
    file.write_all(png_bytes)?;
    file.flush()?;
    file.sync_all()?;
    drop(file);
    if path.is_file() {
        let _ = fs::remove_file(&tmp);
        touch_cache_file(path);
        return Ok(());
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = path.parent() {
        prune_cache_dir(parent, MAX_DISK_CACHE_BYTES, Some(path))?;
    }
    Ok(())
}

fn touch_cache_file(path: &Path) {
    let now = SystemTime::now();
    let times = FileTimes::new().set_accessed(now).set_modified(now);
    if let Ok(file) = OpenOptions::new().read(true).write(true).open(path) {
        let _ = file.set_times(times);
    }
}

fn prune_cache_dir(
    directory: &Path,
    max_bytes: u64,
    preserve: Option<&Path>,
) -> std::io::Result<()> {
    let mut total = 0_u64;
    let mut candidates = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("png") {
            continue;
        }
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        let size = metadata.len();
        let last_used = metadata
            .modified()
            .or_else(|_| metadata.accessed())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        total = total.saturating_add(size);
        candidates.push((path, size, last_used));
    }
    if total <= max_bytes {
        return Ok(());
    }
    candidates.sort_by_key(|(_, _, last_used)| *last_used);
    for (path, size, _) in candidates {
        if preserve.is_some_and(|current| current == path) {
            continue;
        }
        if fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(size);
        }
        if total <= max_bytes {
            break;
        }
    }
    Ok(())
}

pub fn file_mtime_secs(path: &Path) -> Option<u64> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_names_are_stable_for_same_material() {
        let material = CacheKeyMaterial {
            app_identity: "exe:c:\\apps\\code.exe".into(),
            source_path: Some("c:\\apps\\code.exe".into()),
            source_mtime_secs: Some(123),
            size: 64,
        };
        assert_eq!(cache_file_name(&material), cache_file_name(&material));
    }

    #[test]
    fn cache_names_change_when_mtime_changes() {
        let a = CacheKeyMaterial {
            app_identity: "exe:c:\\apps\\code.exe".into(),
            source_path: Some("c:\\apps\\code.exe".into()),
            source_mtime_secs: Some(123),
            size: 64,
        };
        let mut b = a.clone();
        b.source_mtime_secs = Some(456);
        assert_ne!(cache_file_name(&a), cache_file_name(&b));
    }

    #[test]
    fn cache_names_change_when_size_changes() {
        let a = CacheKeyMaterial {
            app_identity: "exe:c:\\apps\\code.exe".into(),
            source_path: Some("c:\\apps\\code.exe".into()),
            source_mtime_secs: Some(123),
            size: 32,
        };
        let mut b = a.clone();
        b.size = 128;
        assert_ne!(cache_file_name(&a), cache_file_name(&b));
    }

    fn fixture_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "itime-icon-cache-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn corrupt_cache_entry_is_removed() {
        let directory = fixture_dir("corrupt");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("broken.png");
        fs::write(&path, b"not a png").unwrap();

        assert_eq!(read_cached_png(&path), None);
        assert!(!path.exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn capacity_pruning_removes_oldest_entry_and_preserves_current_write() {
        let directory = fixture_dir("prune");
        fs::create_dir_all(&directory).unwrap();
        let oldest = directory.join("oldest.png");
        let newer = directory.join("newer.png");
        let current = directory.join("current.png");
        fs::write(&oldest, vec![1_u8; 40]).unwrap();
        fs::write(&newer, vec![2_u8; 40]).unwrap();
        fs::write(&current, vec![3_u8; 40]).unwrap();
        let old_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10);
        OpenOptions::new()
            .write(true)
            .open(&oldest)
            .unwrap()
            .set_times(FileTimes::new().set_modified(old_time))
            .unwrap();
        let newer_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(20);
        OpenOptions::new()
            .write(true)
            .open(&newer)
            .unwrap()
            .set_times(FileTimes::new().set_modified(newer_time))
            .unwrap();

        prune_cache_dir(&directory, 80, Some(&current)).unwrap();

        assert!(!oldest.exists());
        assert!(newer.exists());
        assert!(current.exists());
        fs::remove_dir_all(directory).unwrap();
    }
}
