mod gdi;
mod handles;
mod shell;
mod shortcuts;

use super::{ExtractError, ExtractRequest, IconSource};
use image::RgbaImage;
use std::path::Path;

fn non_generic(image: RgbaImage, size: u32) -> Option<RgbaImage> {
    (!shell::is_generic_application_icon(&image, size)).then_some(image)
}

fn extract_shortcut(
    shortcut: &shortcuts::ShortcutInfo,
    size: u32,
) -> Option<(RgbaImage, IconSource)> {
    if let Ok(image) = shell::shell_item_image_from_path(&shortcut.path, size) {
        if let Some(image) = non_generic(image, size) {
            return Some((image, IconSource::Shortcut));
        }
    }
    // A .lnk's icon location is attacker-controllable; reject UNC/device paths
    // before `is_file` — the probe itself would trigger SMB authentication.
    if let Some(icon_path) = shortcut
        .icon_path
        .as_deref()
        .filter(|path| crate::icons::identity::is_safe_local_path(path) && path.is_file())
    {
        if let Ok(image) = shell::extract_icon_ex_image_at(icon_path, shortcut.icon_index, size) {
            if let Some(image) = non_generic(image, size) {
                return Some((image, IconSource::Shortcut));
            }
        }
    }
    if let Ok(image) = shell::sh_get_file_info_image(&shortcut.path, size) {
        if let Some(image) = non_generic(image, size) {
            return Some((image, IconSource::Shortcut));
        }
    }
    None
}

pub(super) fn extract_rgba_windows(
    req: &ExtractRequest,
    path: Option<&Path>,
    size: u32,
) -> Result<(RgbaImage, IconSource), ExtractError> {
    if let Some(path) = path {
        if let Some(shortcut) = shortcuts::find_shortcut_for_executable(path, &req.app_identity) {
            if let Some(image) = extract_shortcut(&shortcut, size) {
                return Ok(image);
            }
        }
    }

    // NOTE: no caller-controlled AUMID / package-name lookups here. The IPC
    // surface only carries a logical identity, so `shell:AppsFolder\…` and
    // `GetPackagePathByFullName` probing primitives were removed deliberately.

    if let Some(path) = path {
        if let Ok(image) = shell::extract_icon_ex_image(path, size) {
            if let Some(image) = non_generic(image, size) {
                return Ok((image, IconSource::ExtractIcon));
            }
        }
        if let Ok(image) = shell::shell_item_image_from_path(path, size) {
            if let Some(image) = non_generic(image, size) {
                return Ok((image, IconSource::ShellItem));
            }
        }
        if let Ok(image) = shell::sh_get_file_info_image(path, size) {
            if let Some(image) = non_generic(image, size) {
                return Ok((image, IconSource::ShGetFileInfo));
            }
        }
    }

    if let Some(shortcut) = shortcuts::find_shortcut_by_identity(&req.app_identity) {
        if let Some(image) = extract_shortcut(&shortcut, size) {
            return Ok(image);
        }
    }

    Err(ExtractError::NotFound(format!(
        "no icon source for {}",
        req.app_identity
    )))
}
