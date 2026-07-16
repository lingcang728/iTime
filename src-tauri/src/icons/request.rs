use super::extract::ExtractRequest;
use super::identity::normalize_app_identity;
use super::DEFAULT_ICON_SIZE;

pub(super) struct ExtractRequestInput {
    pub app_identity: Option<String>,
    pub executable_path: Option<String>,
    pub process_id: Option<u32>,
    pub aumid: Option<String>,
    pub package_full_name: Option<String>,
    pub package_family_name: Option<String>,
    pub site_host: Option<String>,
    pub requested_size: Option<u32>,
}

pub(super) fn build_extract_request(input: ExtractRequestInput) -> ExtractRequest {
    let ExtractRequestInput {
        app_identity,
        executable_path,
        process_id,
        aumid,
        package_full_name,
        package_family_name,
        site_host,
        requested_size,
    } = input;
    let _ = process_id;
    let (identity, kind) = normalize_app_identity(
        app_identity.as_deref(),
        executable_path.as_deref(),
        aumid.as_deref(),
        package_full_name.as_deref(),
        package_family_name.as_deref(),
        site_host.as_deref(),
    );
    ExtractRequest {
        app_identity: identity,
        identity_kind: kind,
        executable_path,
        aumid,
        package_full_name,
        size: requested_size.unwrap_or(DEFAULT_ICON_SIZE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::identity::AppIdentityKind;

    #[test]
    fn builds_default_sized_logical_request_when_optional_fields_are_absent() {
        // Given
        let input = ExtractRequestInput {
            app_identity: Some("VS Code".into()),
            executable_path: None,
            process_id: None,
            aumid: None,
            package_full_name: None,
            package_family_name: None,
            site_host: None,
            requested_size: None,
        };

        // When
        let request = build_extract_request(input);

        // Then
        assert_eq!(request.app_identity, "app:vs-code");
        assert_eq!(request.identity_kind, AppIdentityKind::Logical);
        assert_eq!(request.size, DEFAULT_ICON_SIZE);
    }
}
