//! Error definitions for the SDK.

use std::collections::HashMap;
use std::time::Duration;

use http::StatusCode;
use serde_json::Value;
use thiserror::Error;

#[cfg(feature = "mcp")]
use rmcp::service::ServiceError;

use crate::client::RetryMetadata;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP client error: {source}")]
    HttpClient {
        #[from]
        source: reqwest::Error,
    },

    #[error("API error (status {status}): {message}")]
    ApiError {
        status: u16,
        message: String,
        code: Option<String>,
        details: Option<Value>,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        retry_after_secs: Option<u64>,
        retryable: Option<bool>,
        attempts: Option<u32>,
    },

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Parse error: {message}")]
    Parse { message: String },

    #[error("Serialization error: {source}")]
    Serialization {
        #[from]
        source: serde_json::Error,
    },

    #[error("IO error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    #[error("Timeout: {message}")]
    Timeout { message: String },

    #[error("Missing thought signature: {message}")]
    MissingThoughtSignature { message: String },

    #[error("Auth error: {message}")]
    Auth { message: String },

    #[error("Channel closed")]
    ChannelClosed,

    #[error("WebSocket error: {source}")]
    WebSocket {
        #[from]
        source: tokio_tungstenite::tungstenite::Error,
    },

    #[cfg(feature = "mcp")]
    #[error("MCP error: {source}")]
    Mcp {
        #[from]
        source: ServiceError,
    },
}

impl Error {
    pub(crate) fn api_error(status: u16, message: impl Into<String>) -> Self {
        Self::ApiError {
            status,
            message: message.into(),
            code: None,
            details: None,
            headers: None,
            body: None,
            retry_after_secs: None,
            retryable: Some(default_retryable_status(status)),
            attempts: None,
        }
    }

    pub(crate) async fn api_error_from_response(
        response: reqwest::Response,
        retryable_override: Option<bool>,
    ) -> Self {
        let status = response.status().as_u16();
        let retry_metadata = response.extensions().get::<RetryMetadata>().copied();
        let headers = header_map_to_hash_map(response.headers());
        let retry_after_secs = retry_after_secs(response.headers());
        let body = response.text().await.unwrap_or_default();
        let (message, code, details) = parse_google_error(&body, status);

        Self::ApiError {
            status,
            message,
            code,
            details,
            headers,
            body: if body.is_empty() { None } else { Some(body) },
            retry_after_secs,
            retryable: retryable_override
                .or(retry_metadata.map(|meta| meta.retryable))
                .or(Some(default_retryable_status(status))),
            attempts: retry_metadata.map(|meta| meta.attempts),
        }
    }

    #[must_use]
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::ApiError { status, .. } => StatusCode::from_u16(*status).ok(),
            _ => None,
        }
    }

    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::ApiError { code, .. } => code.as_deref(),
            _ => None,
        }
    }

    #[must_use]
    pub fn details(&self) -> Option<&Value> {
        match self {
            Self::ApiError { details, .. } => details.as_ref(),
            _ => None,
        }
    }

    #[must_use]
    pub fn headers(&self) -> Option<&HashMap<String, String>> {
        match self {
            Self::ApiError { headers, .. } => headers.as_ref(),
            _ => None,
        }
    }

    #[must_use]
    pub fn body(&self) -> Option<&str> {
        match self {
            Self::ApiError { body, .. } => body.as_deref(),
            _ => None,
        }
    }

    #[must_use]
    pub fn attempts(&self) -> Option<u32> {
        match self {
            Self::ApiError { attempts, .. } => *attempts,
            _ => None,
        }
    }

    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::ApiError {
                retry_after_secs, ..
            } => retry_after_secs.map(Duration::from_secs),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::ApiError { status: 429, .. })
    }

    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ApiError {
                retryable: Some(value),
                ..
            } => *value,
            Self::ApiError { status, .. } => default_retryable_status(*status),
            _ => false,
        }
    }
}

fn default_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504)
}

fn header_map_to_hash_map(headers: &reqwest::header::HeaderMap) -> Option<HashMap<String, String>> {
    let mut map = HashMap::new();
    for (name, value) in headers {
        let Ok(value_str) = value.to_str() else {
            continue;
        };
        map.entry(name.as_str().to_string())
            .and_modify(|existing: &mut String| {
                if !existing.is_empty() {
                    existing.push_str(", ");
                }
                existing.push_str(value_str);
            })
            .or_insert_with(|| value_str.to_string());
    }
    (!map.is_empty()).then_some(map)
}

fn retry_after_secs(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
}

fn parse_google_error(body: &str, status: u16) -> (String, Option<String>, Option<Value>) {
    let fallback = if body.trim().is_empty() {
        StatusCode::from_u16(status)
            .ok()
            .and_then(|code| code.canonical_reason().map(str::to_string))
            .unwrap_or_else(|| format!("HTTP {status}"))
    } else {
        body.to_string()
    };

    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return (fallback, None, None);
    };
    let Some(error) = value.get("error") else {
        return (fallback, None, None);
    };

    let message = error
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or(fallback);
    let code = error
        .get("status")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            error
                .get("code")
                .and_then(Value::as_i64)
                .map(|value| value.to_string())
        });
    let details = error.get("details").cloned();

    (message, code, details)
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_google_error_extracts_metadata() {
        let body = json!({
            "error": {
                "message": "quota exceeded",
                "status": "RESOURCE_EXHAUSTED",
                "details": [{"kind": "quota"}]
            }
        })
        .to_string();
        let (message, code, details) = parse_google_error(&body, 429);

        assert_eq!(message, "quota exceeded");
        assert_eq!(code.as_deref(), Some("RESOURCE_EXHAUSTED"));
        assert_eq!(details, Some(json!([{"kind": "quota"}])));
    }

    #[test]
    fn parse_google_error_falls_back_to_body() {
        let body = "plain-text failure";
        let (message, code, details) = parse_google_error(body, 500);

        assert_eq!(message, body);
        assert!(code.is_none());
        assert!(details.is_none());
    }
}
