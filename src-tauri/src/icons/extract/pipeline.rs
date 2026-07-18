use super::{ExtractError, ExtractRequest, ExtractedIcon, IconSource};
use crate::icons::cache::{self, CacheKeyMaterial};
use crate::icons::known_apps;
use image::RgbaImage;
use std::path::PathBuf;

fn cache_material(req: &ExtractRequest) -> (Option<PathBuf>, CacheKeyMaterial) {
    let size = req.size.clamp(16, 256);
    let resolved_path = resolve_source_path(req);
    let source_path = resolved_path
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());
    let source_mtime_secs = resolved_path
        .as_ref()
        .and_then(|path| cache::file_mtime_secs(path));
    let material = CacheKeyMaterial {
        app_identity: req.app_identity.clone(),
        source_path,
        source_mtime_secs,
        size,
    };
    (resolved_path, material)
}

/// Disk-cache lookup only. Never calls Shell / GDI.
pub fn try_cache_hit(req: &ExtractRequest) -> Option<ExtractedIcon> {
    let size = req.size.clamp(16, 256);
    let (_path, material) = cache_material(req);
    let cache_path = cache::cache_path_for(&material);
    cache::read_cached_png(&cache_path).map(|hit| ExtractedIcon {
        png_path: hit,
        source: IconSource::Cache,
        width: size,
        height: size,
    })
}

/// Full extraction path. Intended for background worker threads.
pub fn extract_and_cache(req: &ExtractRequest) -> Result<ExtractedIcon, ExtractError> {
    if let Some(hit) = try_cache_hit(req) {
        return Ok(hit);
    }

    let size = req.size.clamp(16, 256);
    let (resolved_path, material) = cache_material(req);
    let cache_path = cache::cache_path_for(&material);
    cache::ensure_cache_dir().map_err(|error| ExtractError::Io(error.to_string()))?;

    #[cfg(windows)]
    {
        let (rgba, source) =
            super::windows::extract_rgba_windows(req, resolved_path.as_deref(), size)?;
        let png = encode_png(&rgba).map_err(ExtractError::Encode)?;
        cache::write_cached_png(&cache_path, &png)
            .map_err(|error| ExtractError::Io(error.to_string()))?;
        Ok(ExtractedIcon {
            png_path: cache_path,
            source,
            width: rgba.width(),
            height: rgba.height(),
        })
    }

    #[cfg(not(windows))]
    {
        let _ = (req, resolved_path, size);
        Err(ExtractError::Api(
            "icon extraction is only implemented on Windows".into(),
        ))
    }
}

pub(super) fn resolve_source_path(req: &ExtractRequest) -> Option<PathBuf> {
    if let Some(path) = req.executable_path.as_deref() {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
        if let Some(known) = known_apps::resolve_known_executable(path) {
            return Some(known);
        }
    }

    #[cfg(windows)]
    if let Some(path) = req.process_id.and_then(process_executable_path) {
        if path.is_file() {
            return Some(path);
        }
    }

    let logical = req
        .app_identity
        .strip_prefix("app:")
        .unwrap_or(req.app_identity.as_str());
    known_apps::resolve_known_executable(logical)
}

#[cfg(windows)]
struct OwnedProcess(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        use windows::Win32::Foundation::CloseHandle;
        // SAFETY: this wrapper exclusively owns a valid OpenProcess handle.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(windows)]
fn process_executable_path(process_id: u32) -> Option<PathBuf> {
    use windows::{
        core::PWSTR,
        Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
    };
    if process_id == 0 {
        return None;
    }
    // SAFETY: the caller supplies a PID only; OpenProcess validates it and returns an owned handle.
    let process = OwnedProcess(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?
    });
    let mut buffer = vec![0_u16; 32_768];
    let mut length = u32::try_from(buffer.len()).ok()?;
    // SAFETY: buffer is uniquely owned and its writable capacity is provided in length.
    unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .ok()?;
    let used = usize::try_from(length).ok()?.min(buffer.len());
    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..used])))
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut bytes);
    image
        .write_with_encoder(encoder)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}
