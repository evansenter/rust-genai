/// Represents the API version to target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiVersion {
    /// V1 Alpha API version (reserved for future use).
    /// Kept for API completeness and tested in unit tests.
    #[allow(dead_code)] // Used in tests, reserved for future API versions
    V1Alpha,
    /// V1 Beta API version (current)
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

/// Header name for API key authentication.
///
/// Using header-based authentication is more secure than query parameters because:
/// - API keys don't appear in server logs, proxy logs, or browser history
/// - Keys are not leaked in error messages containing URLs
/// - Matches Google Cloud API best practices
pub const API_KEY_HEADER: &str = "X-Goog-Api-Key";

/// Represents different API endpoints for the Interactions API
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // "Interaction" suffix is intentional for API clarity
pub enum Endpoint<'a> {
    /// Create a new interaction
    CreateInteraction { stream: bool },
    /// Retrieve an interaction by ID
    GetInteraction { id: &'a str },
    /// Delete an interaction by ID
    DeleteInteraction { id: &'a str },
    /// Cancel a background interaction by ID
    CancelInteraction { id: &'a str },
}

impl Endpoint<'_> {
    /// Constructs the URL path for this endpoint
    fn to_path(&self, version: ApiVersion) -> String {
        match self {
            Self::CreateInteraction { .. } => {
                format!("/{}/interactions", version.as_str())
            }
            Self::GetInteraction { id } => {
                format!("/{}/interactions/{}", version.as_str(), id)
            }
            Self::DeleteInteraction { id } => {
                format!("/{}/interactions/{}", version.as_str(), id)
            }
            Self::CancelInteraction { id } => {
                format!("/{}/interactions/{}/cancel", version.as_str(), id)
            }
        }
    }

    /// Returns whether this endpoint requires SSE parameters
    const fn requires_sse(&self) -> bool {
        match self {
            Self::CreateInteraction { stream } => *stream,
            Self::GetInteraction { .. }
            | Self::DeleteInteraction { .. }
            | Self::CancelInteraction { .. } => false,
        }
    }
}

/// Constructs a URL for a specific endpoint.
///
/// Note: API key authentication is handled via the `X-Goog-Api-Key` header,
/// not as a query parameter. Use [`API_KEY_HEADER`] when making requests.
#[must_use]
pub fn construct_endpoint_url(endpoint: Endpoint) -> String {
    let version = ApiVersion::V1Beta; // Default version for new function
    let path = endpoint.to_path(version);
    let sse_param = if endpoint.requires_sse() {
        "?alt=sse"
    } else {
        ""
    };

    format!("{BASE_URL_PREFIX}{path}{sse_param}")
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
        let url = construct_endpoint_url(endpoint);

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/interactions"
        );
        assert!(!url.contains("alt=sse"));
        assert!(!url.contains("key=")); // API key should not be in URL
    }

    #[test]
    fn test_endpoint_create_interaction_streaming() {
        let endpoint = Endpoint::CreateInteraction { stream: true };
        let url = construct_endpoint_url(endpoint);

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/interactions?alt=sse"
        );
        assert!(url.contains("alt=sse"));
        assert!(!url.contains("key=")); // API key should not be in URL
    }

    #[test]
    fn test_endpoint_get_interaction() {
        let endpoint = Endpoint::GetInteraction {
            id: "interaction-123",
        };
        let url = construct_endpoint_url(endpoint);

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/interactions/interaction-123"
        );
        assert!(url.contains("/interactions/interaction-123"));
        assert!(!url.contains("alt=sse"));
        assert!(!url.contains("key=")); // API key should not be in URL
    }

    #[test]
    fn test_endpoint_delete_interaction() {
        let endpoint = Endpoint::DeleteInteraction {
            id: "interaction-456",
        };
        let url = construct_endpoint_url(endpoint);

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/interactions/interaction-456"
        );
        assert!(url.contains("/interactions/interaction-456"));
        assert!(!url.contains("alt=sse"));
        assert!(!url.contains("key=")); // API key should not be in URL
    }

    #[test]
    fn test_api_key_header_constant() {
        assert_eq!(API_KEY_HEADER, "X-Goog-Api-Key");
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

    #[test]
    fn test_endpoint_cancel_interaction() {
        let endpoint = Endpoint::CancelInteraction {
            id: "interaction-789",
        };
        let url = construct_endpoint_url(endpoint);

        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/interactions/interaction-789/cancel"
        );
        assert!(url.contains("/interactions/interaction-789/cancel"));
        assert!(!url.contains("alt=sse"));
        assert!(!url.contains("key=")); // API key should not be in URL
    }

    #[test]
    fn test_cancel_interaction_requires_sse() {
        assert!(!Endpoint::CancelInteraction { id: "test" }.requires_sse());
    }
}
