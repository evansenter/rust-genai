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
    let action = if stream { "streamGenerateContent" } else { "generateContent" };
    let sse_param = if stream { "&alt=sse" } else { "" };
    format!(
        "{BASE_URL_PREFIX}/{version_str}/models/{model_name}:{action}?key={api_key}{sse_param}",
        version_str = version.as_str()
    )
} 