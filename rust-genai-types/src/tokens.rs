use serde::{Deserialize, Serialize};

use crate::http::HttpOptions;
use crate::live_types::LiveConnectConfig;

/// Ephemeral auth token response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthToken {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Live connect constraints for auth token creation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveConnectConstraints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<LiveConnectConfig>,
}

/// Create auth token config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateAuthTokenConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// Optional. Absolute expire time (RFC3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<String>,
    /// Optional. Reject new sessions after this time (RFC3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_session_expire_time: Option<String>,
    /// Optional. Max usage count. Zero means unlimited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses: Option<i32>,
    /// Optional. Live API constraints locked into the token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_connect_constraints: Option<LiveConnectConstraints>,
    /// Optional. Additional fields to lock in field mask.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_additional_fields: Option<Vec<String>>,
}
