use std::path::{Component, Path, Prefix};

/// Maximum accepted length for a caller-supplied identity seed.
/// Longer strings are truncated before normalization so the IPC surface stays bounded.
const MAX_IDENTITY_SEED_CHARS: usize = 160;

/// Build a stable, opaque application identity string from the caller-supplied
/// logical name / previously issued `app:` identity.
///
/// The IPC boundary only accepts logical identities: the resolver derives real
/// executable paths exclusively from collector-registered `path_hints`, so a
/// compromised WebView can never turn this command into a file-existence oracle,
/// a PID probe, or a UNC/SMB authentication trigger.
///
/// Normalization rules (shared with `domain/appIdentity.ts`):
/// - a leading `app:` prefix is treated as an already-normalized identity
/// - ASCII alphanumerics are kept lowercased
/// - every run of other characters collapses to a single `-`
/// - leading/trailing `-` are trimmed
pub fn normalize_app_identity(app_identity: Option<&str>) -> String {
    let raw = app_identity.map(str::trim).filter(|v| !v.is_empty());
    let Some(seed) = raw else {
        return "app:unknown".to_string();
    };
    let seed = seed.strip_prefix("app:").unwrap_or(seed);
    match normalize_logical_key(Some(seed)) {
        Some(logical) => format!("app:{logical}"),
        None => "app:unknown".to_string(),
    }
}

fn normalize_logical_key(value: Option<&str>) -> Option<String> {
    let raw = value?.trim();
    if raw.is_empty() {
        return None;
    }
    let mut output = String::with_capacity(raw.len());
    let mut pending_separator = false;
    for character in raw.chars().take(MAX_IDENTITY_SEED_CHARS) {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !output.is_empty() {
                output.push('-');
            }
            pending_separator = false;
            output.push(character.to_ascii_lowercase());
        } else {
            pending_separator = true;
        }
    }
    (!output.is_empty()).then_some(output)
}

/// Icon extraction must never touch network or device-namespace paths: probing a
/// `\\host\share\…` UNC path through `is_file`/Shell APIs would trigger SMB and
/// leak NTLMv2 hashes. Only plain `X:\…` drive paths are acceptable.
pub(crate) fn is_safe_local_path(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    match path.components().next() {
        Some(Component::Prefix(prefix)) => matches!(prefix.kind(), Prefix::Disk(_)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn collapses_separator_runs_like_the_frontend() {
        // Frontend normalizeLogicalKey folds runs; the collector's logical_key
        // does the same — the icon identity must agree so path hints hit.
        assert_eq!(normalize_app_identity(Some("QQ 音乐")), "app:qq");
        assert_eq!(normalize_app_identity(Some("VS Code")), "app:vs-code");
        assert_eq!(normalize_app_identity(Some("app - x")), "app:app-x");
    }

    #[test]
    fn keeps_issued_app_identities_idempotent() {
        assert_eq!(normalize_app_identity(Some("app:vscode")), "app:vscode");
    }

    #[test]
    fn sanitizes_hostile_identity_input() {
        // `\`/`:`/`/` all collapse — the result can never be reparsed as a path.
        assert_eq!(
            normalize_app_identity(Some(r"\\evil.example\share\x.exe")),
            "app:evil-example-share-x-exe"
        );
        assert_eq!(
            normalize_app_identity(Some("site:host@browser")),
            "app:site-host-browser"
        );
    }

    #[test]
    fn falls_back_to_unknown() {
        assert_eq!(normalize_app_identity(None), "app:unknown");
        assert_eq!(normalize_app_identity(Some("   ")), "app:unknown");
        assert_eq!(normalize_app_identity(Some("中文应用")), "app:unknown");
    }

    #[test]
    fn caps_identity_seed_length() {
        let long = "a".repeat(10_000);
        let identity = normalize_app_identity(Some(&long));
        assert_eq!(identity.len(), 4 + MAX_IDENTITY_SEED_CHARS);
    }

    #[test]
    fn rejects_unc_and_device_paths() {
        assert!(!is_safe_local_path(Path::new(r"\\host\share\app.exe")));
        assert!(!is_safe_local_path(Path::new(r"\\?\C:\Apps\app.exe")));
        assert!(!is_safe_local_path(Path::new(r"\\.\pipe\x")));
        assert!(!is_safe_local_path(Path::new("relative/app.exe")));
        assert!(!is_safe_local_path(Path::new("//host/share/app.exe")));
        assert!(is_safe_local_path(Path::new(r"C:\Apps\app.exe")));
        assert!(is_safe_local_path(&PathBuf::from(r"D:\tools\x.exe")));
    }
}
