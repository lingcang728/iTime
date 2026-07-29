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
    if let Some(icon_path) = shortcut.icon_path.as_deref().filter(|path| path.is_file()) {
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

    if let Some(aumid) = req.aumid.as_deref().filter(|value| !value.is_empty()) {
        let parsing_name = format!("shell:AppsFolder\\{aumid}");
        if let Ok(image) = shell::shell_item_image_from_parsing_name(&parsing_name, size) {
            return Ok((image, IconSource::ShellItem));
        }
    }

    if let Some(full_name) = req
        .package_full_name
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        if let Some(package_path) = shell::package_path_by_full_name(full_name) {
            if let Ok(image) = shell::shell_item_image_from_path(&package_path, size) {
                return Ok((image, IconSource::PackageAsset));
            }
        }
    }

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
        "no icon source for {}{}",
        req.app_identity,
        req.package_family_name
            .as_deref()
            .map(|family| format!(" ({family})"))
            .unwrap_or_default()
    )))
}
