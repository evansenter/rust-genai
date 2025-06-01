/// Represents the API version to target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiVersion {
    V1Alpha,
    V1Beta,
}

impl ApiVersion {
    const fn as_str(self) -> &'static str {
        match self {
            Self::V1Alpha => "v1alpha",
            Self::V1Beta => "v1beta",
        }
    }
}

// --- URL Construction ---
const BASE_URL_PREFIX: &str = "https://generativelanguage.googleapis.com";

#[must_use]
pub fn construct_url(model_name: &str, api_key: &str, stream: bool, version: ApiVersion) -> String {
    let action = if stream {
        "streamGenerateContent"
    } else {
        "generateContent"
    };
    let sse_param = if stream { "&alt=sse" } else { "" };
    format!(
        "{BASE_URL_PREFIX}/{version_str}/models/{model_name}:{action}?key={api_key}{sse_param}",
        version_str = version.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_as_str() {
        assert_eq!(ApiVersion::V1Alpha.as_str(), "v1alpha");
        assert_eq!(ApiVersion::V1Beta.as_str(), "v1beta");
    }

    #[test]
    fn test_construct_url_non_streaming() {
        let url = construct_url("gemini-pro", "test-key", false, ApiVersion::V1Alpha);
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1alpha/models/gemini-pro:generateContent?key=test-key"
        );

        let url = construct_url("gemini-pro", "test-key", false, ApiVersion::V1Beta);
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key=test-key"
        );
    }

    #[test]
    fn test_construct_url_streaming() {
        let url = construct_url("gemini-pro", "test-key", true, ApiVersion::V1Alpha);
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1alpha/models/gemini-pro:streamGenerateContent?key=test-key&alt=sse"
        );

        let url = construct_url("gemini-pro", "test-key", true, ApiVersion::V1Beta);
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:streamGenerateContent?key=test-key&alt=sse"
        );
    }

    #[test]
    fn test_construct_url_different_models() {
        let models = vec![
            "gemini-pro",
            "gemini-1.5-flash",
            "gemini-2.5-flash-preview-05-20",
            "test-model-123",
        ];

        for model in models {
            let url = construct_url(model, "api-key", false, ApiVersion::V1Alpha);
            assert!(url.contains(model));
            assert!(url.contains("generateContent"));
            assert!(!url.contains("&alt=sse"));
        }
    }

    #[test]
    fn test_construct_url_special_characters_in_model_name() {
        // Test URL encoding is handled by the HTTP client, not this function
        let url = construct_url("model-with-special_chars.v1", "key", false, ApiVersion::V1Beta);
        assert!(url.contains("model-with-special_chars.v1"));
    }
}
