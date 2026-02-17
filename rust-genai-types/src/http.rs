use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP retry options to be used in each of the requests.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HttpRetryOptions {
    /// Maximum number of attempts, including the original request.
    /// If 0 or 1, it means no retries. If not specified, defaults to 5.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempts: Option<u32>,
    /// Initial delay before the first retry, in seconds. Defaults to 1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_delay: Option<f64>,
    /// Maximum delay between retries, in seconds. Defaults to 60.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_delay: Option<f64>,
    /// Multiplier by which the delay increases after each attempt. Defaults to 2.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_base: Option<f64>,
    /// Randomness factor for the delay, in seconds. Defaults to 1.0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jitter: Option<f64>,
    /// List of HTTP status codes that should trigger a retry.
    ///
    /// If not specified, a default set of retryable codes (408, 429, and select 5xx) is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status_codes: Option<Vec<u16>>,
}

/// HTTP options to be used in each of the requests.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HttpOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Timeout for the request in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// Extra parameters to add to the request body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_body: Option<serde_json::Value>,
    /// HTTP retry options for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_options: Option<HttpRetryOptions>,
}
