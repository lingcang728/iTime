use crate::icons::ICON_RESOLVER_VERSION;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
        return None;
    }
    // Quick PNG signature check
    let mut header = [0u8; 8];
    if let Ok(mut file) = fs::File::open(path) {
        use std::io::Read;
        if file.read_exact(&mut header).is_ok() && &header == b"\x89PNG\r\n\x1a\n" {
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
    fs::write(&tmp, png_bytes)?;
    if path.is_file() {
        let _ = fs::remove_file(&tmp);
        return Ok(());
    }
    fs::rename(&tmp, path)?;
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
}
