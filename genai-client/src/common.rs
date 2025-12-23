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

/// Represents different API endpoints for the Interactions API
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint<'a> {
    /// Create a new interaction
    CreateInteraction { stream: bool },
    /// Retrieve an interaction by ID
    GetInteraction { id: &'a str },
    /// Delete an interaction by ID
    DeleteInteraction { id: &'a str },
}

impl Endpoint<'_> {
    /// Constructs the URL path for this endpoint
    fn to_path(&self, version: ApiVersion) -> String {
        match self {
            Self::CreateInteraction { stream: _ } => {
                format!("/{}/interactions", version.as_str())
            }
            Self::GetInteraction { id } => {
                format!("/{}/interactions/{}", version.as_str(), id)
            }
            Self::DeleteInteraction { id } => {
                format!("/{}/interactions/{}", version.as_str(), id)
            }
        }
    }

    /// Returns whether this endpoint requires SSE parameters
    const fn requires_sse(&self) -> bool {
        match self {
            Self::CreateInteraction { stream } => *stream,
            Self::GetInteraction { .. } | Self::DeleteInteraction { .. } => false,
        }
    }
}

/// Constructs a URL for a specific endpoint
#[must_use]
pub fn construct_endpoint_url(endpoint: Endpoint, api_key: &str) -> String {
    let version = ApiVersion::V1Beta; // Default version for new function
    let path = endpoint.to_path(version);
    let sse_param = if endpoint.requires_sse() {
        "&alt=sse"
    } else {
        ""
    };

    format!("{BASE_URL_PREFIX}{path}?key={api_key}{sse_param}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_as_str() {
        assert_eq!(ApiVersion::V1Alpha.as_str(), "v1alpha");
        assert_eq!(ApiVersion::V1Beta.as_str(), "v1beta");
    }

    // --- Tests for Endpoint-based URL construction ---

    #[test]
    fn test_endpoint_create_interaction_non_streaming() {
        let endpoint = Endpoint::CreateInteraction { stream: false };
        let url = construct_endpoint_url(endpoint, "my-key");

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/interactions?key=my-key"
        );
        assert!(!url.contains("&alt=sse"));
    }

    #[test]
    fn test_endpoint_create_interaction_streaming() {
        let endpoint = Endpoint::CreateInteraction { stream: true };
        let url = construct_endpoint_url(endpoint, "my-key");

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/interactions?key=my-key&alt=sse"
        );
        assert!(url.contains("&alt=sse"));
    }

    #[test]
    fn test_endpoint_get_interaction() {
        let endpoint = Endpoint::GetInteraction {
            id: "interaction-123",
        };
        let url = construct_endpoint_url(endpoint, "api-key");

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/interactions/interaction-123?key=api-key"
        );
        assert!(url.contains("/interactions/interaction-123"));
        assert!(!url.contains("&alt=sse"));
    }

    #[test]
    fn test_endpoint_delete_interaction() {
        let endpoint = Endpoint::DeleteInteraction {
            id: "interaction-456",
        };
        let url = construct_endpoint_url(endpoint, "api-key");

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/interactions/interaction-456?key=api-key"
        );
        assert!(url.contains("/interactions/interaction-456"));
        assert!(!url.contains("&alt=sse"));
    }

    #[test]
    fn test_endpoint_requires_sse() {
        assert!(Endpoint::CreateInteraction { stream: true }.requires_sse());
        assert!(!Endpoint::CreateInteraction { stream: false }.requires_sse());
        assert!(!Endpoint::GetInteraction { id: "test" }.requires_sse());
        assert!(!Endpoint::DeleteInteraction { id: "test" }.requires_sse());
    }

    #[test]
    fn test_endpoint_to_path_with_different_versions() {
        let endpoint = Endpoint::CreateInteraction { stream: false };

        let path_v1alpha = endpoint.to_path(ApiVersion::V1Alpha);
        assert_eq!(path_v1alpha, "/v1alpha/interactions");

        let path_v1beta = endpoint.to_path(ApiVersion::V1Beta);
        assert_eq!(path_v1beta, "/v1beta/interactions");
    }

    #[test]
    fn test_endpoint_clone_and_eq() {
        let endpoint1 = Endpoint::CreateInteraction { stream: true };
        let endpoint2 = endpoint1.clone();
        assert_eq!(endpoint1, endpoint2);

        let endpoint3 = Endpoint::GetInteraction { id: "test-id" };
        let endpoint4 = Endpoint::GetInteraction { id: "test-id" };
        assert_eq!(endpoint3, endpoint4);

        let endpoint5 = Endpoint::GetInteraction { id: "different-id" };
        assert_ne!(endpoint3, endpoint5);
    }
}
