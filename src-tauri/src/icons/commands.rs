use super::queue::{IconService, IconUpdateEvent};
use super::request::{build_extract_request, ExtractRequestInput};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

/// Security boundary: the WebView may only name a logical app identity and a
/// size. Concrete `executablePath`/`processId`/`aumid`/`package*`/`siteHost`
/// inputs were removed on purpose — they made this command a file-existence
/// oracle, a PID/package probe, and (via UNC paths) an NTLMv2 hash exfiltration
/// primitive. Unknown fields are rejected so a poisoned frontend fails loudly
/// instead of being silently ignored.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IconResolveRequest {
    pub app_identity: Option<String>,
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
        requested_size: request.requested_size,
    });
    icons.try_get_cached_or_enqueue(app, extract).into()
}
