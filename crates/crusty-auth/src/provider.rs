//! Authentication providers that can apply auth to a request.

use crate::error::AuthError;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for request authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthConfig {
    /// No authentication.
    None,
    /// Bearer token authentication.
    Bearer {
        /// The bearer token.
        token: String,
    },
    /// Basic authentication.
    Basic {
        /// Username.
        username: String,
        /// Password.
        password: String,
    },
    /// API key authentication.
    ApiKey {
        /// The key name (header name, query param name, or cookie name).
        key: String,
        /// The key value.
        value: String,
        /// Where to send the API key.
        location: ApiKeyLocation,
    },
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self::None
    }
}

/// Where an API key should be sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    /// As an HTTP header.
    Header,
    /// As a query parameter.
    Query,
}

/// Trait for applying authentication to a request.
pub trait AuthProvider {
    /// Apply authentication, modifying headers and/or query params.
    fn apply(
        &self,
        headers: &mut HashMap<String, String>,
        query_params: &mut Vec<(String, String)>,
    ) -> Result<(), AuthError>;
}

impl AuthProvider for AuthConfig {
    fn apply(
        &self,
        headers: &mut HashMap<String, String>,
        query_params: &mut Vec<(String, String)>,
    ) -> Result<(), AuthError> {
        match self {
            AuthConfig::None => Ok(()),

            AuthConfig::Bearer { token } => {
                if token.is_empty() {
                    return Err(AuthError::MissingField("token".into()));
                }
                headers.insert("Authorization".into(), format!("Bearer {token}"));
                Ok(())
            }

            AuthConfig::Basic { username, password } => {
                let credentials = format!("{username}:{password}");
                let encoded =
                    base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
                headers.insert("Authorization".into(), format!("Basic {encoded}"));
                Ok(())
            }

            AuthConfig::ApiKey {
                key,
                value,
                location,
            } => {
                if key.is_empty() {
                    return Err(AuthError::MissingField("key".into()));
                }
                match location {
                    ApiKeyLocation::Header => {
                        headers.insert(key.clone(), value.clone());
                    }
                    ApiKeyLocation::Query => {
                        query_params.push((key.clone(), value.clone()));
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bearer_auth() {
        let auth = AuthConfig::Bearer {
            token: "my-secret-token".into(),
        };
        let mut headers = HashMap::new();
        let mut params = Vec::new();
        auth.apply(&mut headers, &mut params).unwrap();

        assert_eq!(
            headers.get("Authorization").unwrap(),
            "Bearer my-secret-token"
        );
    }

    #[test]
    fn test_basic_auth() {
        let auth = AuthConfig::Basic {
            username: "user".into(),
            password: "pass".into(),
        };
        let mut headers = HashMap::new();
        let mut params = Vec::new();
        auth.apply(&mut headers, &mut params).unwrap();

        let expected = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("user:pass")
        );
        assert_eq!(headers.get("Authorization").unwrap(), &expected);
    }

    #[test]
    fn test_api_key_header() {
        let auth = AuthConfig::ApiKey {
            key: "X-API-Key".into(),
            value: "abc123".into(),
            location: ApiKeyLocation::Header,
        };
        let mut headers = HashMap::new();
        let mut params = Vec::new();
        auth.apply(&mut headers, &mut params).unwrap();

        assert_eq!(headers.get("X-API-Key").unwrap(), "abc123");
    }

    #[test]
    fn test_api_key_query() {
        let auth = AuthConfig::ApiKey {
            key: "api_key".into(),
            value: "secret".into(),
            location: ApiKeyLocation::Query,
        };
        let mut headers = HashMap::new();
        let mut params = Vec::new();
        auth.apply(&mut headers, &mut params).unwrap();

        assert!(headers.is_empty());
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], ("api_key".into(), "secret".into()));
    }

    #[test]
    fn test_none_auth() {
        let auth = AuthConfig::None;
        let mut headers = HashMap::new();
        let mut params = Vec::new();
        auth.apply(&mut headers, &mut params).unwrap();

        assert!(headers.is_empty());
        assert!(params.is_empty());
    }

    #[test]
    fn test_empty_bearer_token_error() {
        let auth = AuthConfig::Bearer {
            token: String::new(),
        };
        let mut headers = HashMap::new();
        let mut params = Vec::new();
        assert!(auth.apply(&mut headers, &mut params).is_err());
    }
}
