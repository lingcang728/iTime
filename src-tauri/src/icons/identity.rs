use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppIdentityKind {
    ExecutablePath,
    Package,
    BrowserSite,
    Logical,
}

/// Build a stable application identity string.
/// Prefer package / AUMID, then normalized absolute exe path, then logical key.
pub fn normalize_app_identity(
    app_identity: Option<&str>,
    executable_path: Option<&str>,
    aumid: Option<&str>,
    package_full_name: Option<&str>,
    package_family_name: Option<&str>,
    site_host: Option<&str>,
) -> (String, AppIdentityKind) {
    if let Some(host) = site_host.map(str::trim).filter(|v| !v.is_empty()) {
        let browser = normalize_path_key(executable_path)
            .map(|path| private_path_key(&path))
            .or_else(|| normalize_logical_key(app_identity))
            .unwrap_or_else(|| "browser".to_string());
        return (
            format!("site:{}@{}", host.to_ascii_lowercase(), browser),
            AppIdentityKind::BrowserSite,
        );
    }

    if let Some(aumid) = aumid.map(str::trim).filter(|v| !v.is_empty()) {
        return (format!("aumid:{}", aumid), AppIdentityKind::Package);
    }
    if let Some(full) = package_full_name.map(str::trim).filter(|v| !v.is_empty()) {
        return (format!("pkg:{}", full), AppIdentityKind::Package);
    }
    if let Some(family) = package_family_name.map(str::trim).filter(|v| !v.is_empty()) {
        return (format!("pkgfamily:{}", family), AppIdentityKind::Package);
    }

    if let Some(path_key) = normalize_path_key(executable_path) {
        return (
            format!("exe:{}", private_path_key(&path_key)),
            AppIdentityKind::ExecutablePath,
        );
    }

    if let Some(logical) = normalize_logical_key(app_identity) {
        return (format!("app:{}", logical), AppIdentityKind::Logical);
    }

    ("app:unknown".to_string(), AppIdentityKind::Logical)
}

fn private_path_key(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub fn normalize_path_key(path: Option<&str>) -> Option<String> {
    let raw = path?.trim();
    if raw.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|_| normalize_slashes(&path));
    Some(
        canonical
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_ascii_lowercase()
            .replace('/', "\\"),
    )
}

fn normalize_slashes(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace('/', "\\"))
}

fn normalize_logical_key(value: Option<&str>) -> Option<String> {
    let raw = value?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(
        raw.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string(),
    )
    .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_aumid_over_path() {
        let (id, kind) = normalize_app_identity(
            Some("chrome"),
            Some(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
            Some("Chrome.App"),
            None,
            None,
            None,
        );
        assert_eq!(kind, AppIdentityKind::Package);
        assert_eq!(id, "aumid:Chrome.App");
    }

    #[test]
    fn builds_browser_site_identity() {
        let (id, kind) = normalize_app_identity(
            Some("chrome"),
            Some(r"C:\Chrome\chrome.exe"),
            None,
            None,
            None,
            Some("github.com"),
        );
        assert_eq!(kind, AppIdentityKind::BrowserSite);
        assert!(id.starts_with("site:github.com@"));
    }

    #[test]
    fn executable_identity_never_exposes_the_source_path() {
        let (id, kind) = normalize_app_identity(
            None,
            Some(r"C:\Users\person\Apps\Secret\tool.exe"),
            None,
            None,
            None,
            None,
        );
        assert_eq!(kind, AppIdentityKind::ExecutablePath);
        assert!(id.starts_with("exe:"));
        assert_eq!(id.len(), 20);
        assert!(!id.to_ascii_lowercase().contains("users"));
        assert!(!id.to_ascii_lowercase().contains("secret"));
    }

    #[test]
    fn falls_back_to_logical_key() {
        let (id, kind) = normalize_app_identity(Some("VS Code"), None, None, None, None, None);
        assert_eq!(kind, AppIdentityKind::Logical);
        assert_eq!(id, "app:vs-code");
    }
}
