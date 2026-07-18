use super::*;
use crate::icons::identity::AppIdentityKind;

#[test]
fn extracts_known_vscode_icon_when_installed() {
    // Given
    let req = vscode_request();
    let path = pipeline::resolve_source_path(&req);
    if path.is_none() {
        eprintln!("skip: VS Code not installed on this machine");
        return;
    }

    // When
    let icon = extract_and_cache(&req).expect("vscode icon extract");
    let again = extract_and_cache(&req).expect("cache hit");

    // Then
    assert!(icon.png_path.is_file());
    assert!(icon.width >= 16);
    assert_eq!(again.source, IconSource::Cache);
    assert_eq!(again.png_path, icon.png_path);
}

#[test]
fn extracts_windows_rgba_when_known_vscode_path_is_installed() {
    // Given
    let req = vscode_request();
    let Some(path) = pipeline::resolve_source_path(&req) else {
        eprintln!("skip: VS Code not installed on this machine");
        return;
    };

    // When
    let (image, source) = windows::extract_rgba_windows(&req, Some(&path), 48)
        .expect("direct Windows icon extraction");

    // Then
    assert!(image.width() >= 16);
    assert!(image.height() >= 16);
    assert_ne!(source, IconSource::Cache);
    assert_ne!(source, IconSource::Fallback);
}

#[test]
fn extracts_an_installed_windows_shortcut_by_logical_name() {
    for logical in ["notion", "microsoft-word", "antigravity", "wechat"] {
        let req = ExtractRequest {
            app_identity: format!("app:{logical}"),
            identity_kind: AppIdentityKind::Logical,
            executable_path: None,
            process_id: None,
            aumid: None,
            package_full_name: None,
            package_family_name: None,
            size: 48,
        };
        if let Ok((image, source)) = windows::extract_rgba_windows(&req, None, 48) {
            assert!(image.width() >= 16);
            assert_eq!(source, IconSource::Shortcut);
            return;
        }
    }
    eprintln!("skip: no matching application shortcut installed on this machine");
}

fn vscode_request() -> ExtractRequest {
    ExtractRequest {
        app_identity: "app:vscode".into(),
        identity_kind: AppIdentityKind::Logical,
        executable_path: None,
        process_id: None,
        aumid: None,
        package_full_name: None,
        package_family_name: None,
        size: 64,
    }
}
