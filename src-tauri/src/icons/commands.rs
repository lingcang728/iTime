use super::queue::{IconService, IconUpdateEvent};
use super::request::{build_extract_request, ExtractRequestInput};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconResolveRequest {
    pub app_identity: Option<String>,
    pub executable_path: Option<String>,
    pub process_id: Option<u32>,
    pub aumid: Option<String>,
    pub package_full_name: Option<String>,
    pub package_family_name: Option<String>,
    pub site_host: Option<String>,
    pub requested_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconResolveResponse {
    pub app_identity: String,
    pub status: String,
    pub cache_path: Option<String>,
    pub icon_source: String,
    pub width: u32,
    pub height: u32,
    pub error_code: Option<String>,
}

impl From<IconUpdateEvent> for IconResolveResponse {
    fn from(value: IconUpdateEvent) -> Self {
        Self {
            app_identity: value.app_identity,
            status: value.status,
            cache_path: value.cache_path,
            icon_source: value.icon_source,
            width: value.width,
            height: value.height,
            error_code: value.error_code,
        }
    }
}

#[tauri::command]
pub fn resolve_app_icon(
    app: AppHandle,
    icons: State<'_, IconService>,
    request: IconResolveRequest,
) -> IconResolveResponse {
    let extract = build_extract_request(ExtractRequestInput {
        app_identity: request.app_identity,
        executable_path: request.executable_path,
        process_id: request.process_id,
        aumid: request.aumid,
        package_full_name: request.package_full_name,
        package_family_name: request.package_family_name,
        site_host: request.site_host,
        requested_size: request.requested_size,
    });
    icons.try_get_cached_or_enqueue(app, extract).into()
}

#[tauri::command]
pub fn get_icon_cache_dir() -> String {
    super::cache::icons_cache_dir()
        .to_string_lossy()
        .to_string()
}
