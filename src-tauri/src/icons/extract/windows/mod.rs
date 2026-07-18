mod gdi;
mod handles;
mod shell;
mod shortcuts;

use super::{ExtractError, ExtractRequest, IconSource};
use image::RgbaImage;
use std::path::Path;

pub(super) fn extract_rgba_windows(
    req: &ExtractRequest,
    path: Option<&Path>,
    size: u32,
) -> Result<(RgbaImage, IconSource), ExtractError> {
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

    if let Some(shortcut) = shortcuts::find_shortcut_by_identity(&req.app_identity) {
        if let Ok(image) = shell::shell_item_image_from_path(&shortcut, size) {
            return Ok((image, IconSource::Shortcut));
        }
        if let Ok(image) = shell::sh_get_file_info_image(&shortcut, size) {
            return Ok((image, IconSource::Shortcut));
        }
    }

    if let Some(path) = path {
        if let Ok(image) = shell::shell_item_image_from_path(path, size) {
            return Ok((image, IconSource::ShellItem));
        }
        if let Ok(image) = shell::sh_get_file_info_image(path, size) {
            return Ok((image, IconSource::ShGetFileInfo));
        }
        if let Ok(image) = shell::extract_icon_ex_image(path, size) {
            return Ok((image, IconSource::ExtractIcon));
        }
        if let Some(shortcut) = shortcuts::find_start_menu_shortcut(path) {
            if let Ok(image) = shell::shell_item_image_from_path(&shortcut, size) {
                return Ok((image, IconSource::Shortcut));
            }
            if let Ok(image) = shell::sh_get_file_info_image(&shortcut, size) {
                return Ok((image, IconSource::Shortcut));
            }
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
