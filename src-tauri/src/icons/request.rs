use super::extract::ExtractRequest;
use super::identity::normalize_app_identity;
use super::DEFAULT_ICON_SIZE;

/// IPC-facing request shape. Only the logical identity and a size are accepted:
/// executable paths arrive exclusively through `IconService::register_executable_hint`
/// (collector-observed foreground processes), never from WebView input.
pub(super) struct ExtractRequestInput {
    pub app_identity: Option<String>,
    pub requested_size: Option<u32>,
}

pub(super) fn build_extract_request(input: ExtractRequestInput) -> ExtractRequest {
    ExtractRequest {
        app_identity: normalize_app_identity(input.app_identity.as_deref()),
        executable_path: None,
        size: input
            .requested_size
            .unwrap_or(DEFAULT_ICON_SIZE)
            .clamp(16, 256),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_default_sized_logical_request_when_optional_fields_are_absent() {
        // Given
        let input = ExtractRequestInput {
            app_identity: Some("VS Code".into()),
            requested_size: None,
        };

        // When
        let request = build_extract_request(input);

        // Then
        assert_eq!(request.app_identity, "app:vs-code");
        assert_eq!(request.size, DEFAULT_ICON_SIZE);
        assert_eq!(request.executable_path, None);
    }

    #[test]
    fn clamps_requested_size_to_supported_bounds() {
        let mut input = ExtractRequestInput {
            app_identity: Some("VS Code".into()),
            requested_size: Some(1),
        };
        assert_eq!(build_extract_request(input).size, 16);
        input = ExtractRequestInput {
            app_identity: Some("VS Code".into()),
            requested_size: Some(1_024),
        };
        assert_eq!(build_extract_request(input).size, 256);
    }
}
