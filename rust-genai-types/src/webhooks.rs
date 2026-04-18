use std::collections::HashMap;

use crate::http::HttpOptions;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration for webhook notifications on long-running operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WebhookConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uris: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_metadata: Option<HashMap<String, Value>>,
}

/// Represents a signing secret used to verify webhook payloads.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct SigningSecret {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated_secret: Option<String>,
}

/// A Webhook resource.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct Webhook {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subscribed_events: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_signing_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_secrets: Option<Vec<SigningSecret>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
}

/// Configuration for creating a webhook.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct CreateWebhookConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subscribed_events: Vec<String>,
    pub uri: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub webhook_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl CreateWebhookConfig {
    #[must_use]
    pub fn new(uri: impl Into<String>, subscribed_events: Vec<String>) -> Self {
        Self {
            http_options: None,
            subscribed_events,
            uri: uri.into(),
            webhook_id: None,
            name: None,
            state: None,
        }
    }
}

/// Configuration for updating a webhook.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct UpdateWebhookConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subscribed_events: Vec<String>,
    pub uri: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub update_mask: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl UpdateWebhookConfig {
    #[must_use]
    pub fn new(uri: impl Into<String>, subscribed_events: Vec<String>) -> Self {
        Self {
            http_options: None,
            subscribed_events,
            uri: uri.into(),
            update_mask: None,
            name: None,
            state: None,
        }
    }
}

/// Configuration for listing webhooks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ListWebhooksConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

/// Configuration for deleting a webhook.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DeleteWebhookConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// Configuration for fetching a webhook.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct GetWebhookConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// Configuration for pinging a webhook.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct PingWebhookConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// Configuration for rotating a webhook signing secret.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RotateWebhookSigningSecretConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revocation_behavior: Option<String>,
}

/// Response for listing webhooks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct WebhookListResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhooks: Option<Vec<Webhook>>,
}

/// Response for deleting a webhook.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct WebhookDeleteResponse {}

/// Response for pinging a webhook.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct WebhookPingResponse {}

/// Response for rotating a webhook signing secret.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct WebhookRotateSigningSecretResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}
