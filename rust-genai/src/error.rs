//! Error definitions for the SDK.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime};

use http::StatusCode;
use serde_json::Value;
use thiserror::Error;

#[cfg(feature = "mcp")]
use rmcp::service::ServiceError;

use crate::client::RetryMetadata;

#[derive(Clone, Debug, Default)]
struct ApiErrorMetadata {
    code: Option<String>,
    details: Option<Value>,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    retry_after_secs: Option<u64>,
    retryable: Option<bool>,
    attempts: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ApiErrorKey {
    status: u16,
    message_ptr: usize,
    message_len: usize,
    message_hash: u64,
}

impl ApiErrorKey {
    fn new(status: u16, message: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        message.hash(&mut hasher);
        Self {
            status,
            message_ptr: message.as_ptr() as usize,
            message_len: message.len(),
            message_hash: hasher.finish(),
        }
    }
}

#[derive(Default)]
struct ApiErrorMetadataRegistry {
    entries: HashMap<ApiErrorKey, ApiErrorMetadata>,
    order: VecDeque<ApiErrorKey>,
}

impl ApiErrorMetadataRegistry {
    fn get(&self, key: &ApiErrorKey) -> Option<ApiErrorMetadata> {
        self.entries.get(key).cloned()
    }

    fn insert(&mut self, key: ApiErrorKey, metadata: ApiErrorMetadata) {
        const API_ERROR_METADATA_CAPACITY: usize = 4096;

        if self.entries.insert(key, metadata).is_none() {
            self.order.push_back(key);
        }

        while self.entries.len() > API_ERROR_METADATA_CAPACITY {
            let Some(oldest_key) = self.order.pop_front() else {
                break;
            };
            self.entries.remove(&oldest_key);
        }
    }
}

static API_ERROR_METADATA_REGISTRY: LazyLock<Mutex<ApiErrorMetadataRegistry>> =
    LazyLock::new(|| Mutex::new(ApiErrorMetadataRegistry::default()));

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP client error: {source}")]
    HttpClient {
        #[from]
        source: reqwest::Error,
    },

    #[error("API error (status {status}): {message}")]
    ApiError { status: u16, message: String },

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
    pub(crate) fn api_error_with_retryable(
        status: u16,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        let message = message.into();
        set_api_metadata(
            status,
            &message,
            ApiErrorMetadata {
                retryable: Some(retryable),
                ..Default::default()
            },
        );
        Self::ApiError { status, message }
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
        set_api_metadata(
            status,
            &message,
            ApiErrorMetadata {
                code,
                details,
                headers,
                body: if body.is_empty() { None } else { Some(body) },
                retry_after_secs,
                retryable: retryable_override
                    .or(retry_metadata.map(|meta| meta.retryable))
                    .or(Some(default_retryable_status(status))),
                attempts: retry_metadata.map(|meta| meta.attempts),
            },
        );

        Self::ApiError { status, message }
    }

    fn api_metadata(&self) -> Option<ApiErrorMetadata> {
        match self {
            Self::ApiError { status, message } => api_metadata(*status, message),
            _ => None,
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
    pub fn code(&self) -> Option<String> {
        self.api_metadata().and_then(|metadata| metadata.code)
    }

    #[must_use]
    pub fn details(&self) -> Option<Value> {
        self.api_metadata().and_then(|metadata| metadata.details)
    }

    #[must_use]
    pub fn headers(&self) -> Option<HashMap<String, String>> {
        self.api_metadata().and_then(|metadata| metadata.headers)
    }

    #[must_use]
    pub fn body(&self) -> Option<String> {
        self.api_metadata().and_then(|metadata| metadata.body)
    }

    #[must_use]
    pub fn attempts(&self) -> Option<u32> {
        self.api_metadata().and_then(|metadata| metadata.attempts)
    }

    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        self.api_metadata()
            .and_then(|metadata| metadata.retry_after_secs)
            .map(Duration::from_secs)
    }

    #[must_use]
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::ApiError { status: 429, .. })
    }

    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ApiError { status, .. } => self
                .api_metadata()
                .and_then(|metadata| metadata.retryable)
                .unwrap_or_else(|| default_retryable_status(*status)),
            _ => false,
        }
    }
}

fn default_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504)
}

fn api_metadata(status: u16, message: &str) -> Option<ApiErrorMetadata> {
    api_error_metadata_registry().get(&ApiErrorKey::new(status, message))
}

fn set_api_metadata(status: u16, message: &str, metadata: ApiErrorMetadata) {
    api_error_metadata_registry().insert(ApiErrorKey::new(status, message), metadata);
}

fn api_error_metadata_registry() -> std::sync::MutexGuard<'static, ApiErrorMetadataRegistry> {
    API_ERROR_METADATA_REGISTRY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
    let retry_after = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())?
        .trim();

    retry_after.parse::<u64>().ok().or_else(|| {
        httpdate::parse_http_date(retry_after).ok().map(|deadline| {
            deadline
                .duration_since(SystemTime::now())
                .unwrap_or_default()
                .as_secs()
        })
    })
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
    use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
    use serde_json::json;
    use std::time::SystemTime;

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

    #[test]
    fn api_error_accessors_cover_defaults() {
        let err =
            Error::api_error_with_retryable(503, "unavailable", default_retryable_status(503));
        assert_eq!(err.status(), Some(StatusCode::SERVICE_UNAVAILABLE));
        assert_eq!(err.code(), None);
        assert_eq!(err.details(), None);
        assert_eq!(err.headers(), None);
        assert_eq!(err.body(), None);
        assert_eq!(err.attempts(), None);
        assert_eq!(err.retry_after(), None);
        assert!(err.is_retryable());
        assert!(!err.is_rate_limited());

        let bad_request =
            Error::api_error_with_retryable(400, "bad request", default_retryable_status(400));
        assert_eq!(bad_request.status(), Some(StatusCode::BAD_REQUEST));
        assert!(!bad_request.is_retryable());

        let terminal = Error::api_error_with_retryable(500, "terminal", false);
        assert_eq!(terminal.status(), Some(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!terminal.is_retryable());
    }

    #[test]
    fn api_error_public_shape_stays_constructible() {
        let err = Error::ApiError {
            status: 418,
            message: "teapot".into(),
        };

        assert_eq!(err.status(), Some(StatusCode::IM_A_TEAPOT));
        assert_eq!(err.code(), None);
        assert_eq!(err.details(), None);
        assert_eq!(err.headers(), None);
        assert_eq!(err.body(), None);
        assert_eq!(err.attempts(), None);
        assert_eq!(err.retry_after(), None);
        assert!(!err.is_retryable());
    }

    #[test]
    fn accessors_are_empty_for_non_api_errors() {
        let err = Error::Parse {
            message: "boom".into(),
        };
        assert_eq!(err.status(), None);
        assert_eq!(err.code(), None);
        assert_eq!(err.details(), None);
        assert_eq!(err.headers(), None);
        assert_eq!(err.body(), None);
        assert_eq!(err.attempts(), None);
        assert_eq!(err.retry_after(), None);
        assert!(!err.is_retryable());
        assert!(!err.is_rate_limited());
    }

    #[test]
    fn header_helpers_collect_values_and_retry_after() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test", HeaderValue::from_static("a"));
        headers.append("x-test", HeaderValue::from_static("b"));
        headers.insert(RETRY_AFTER, HeaderValue::from_static("7"));

        let flattened = header_map_to_hash_map(&headers).unwrap();
        assert_eq!(flattened.get("x-test").map(String::as_str), Some("a, b"));
        assert_eq!(retry_after_secs(&headers), Some(7));
    }

    #[test]
    fn retry_after_secs_parses_http_date() {
        let mut headers = HeaderMap::new();
        let deadline = SystemTime::now() + Duration::from_secs(60);
        let header = httpdate::fmt_http_date(deadline);
        headers.insert(RETRY_AFTER, HeaderValue::from_str(&header).unwrap());

        let retry_after = retry_after_secs(&headers).unwrap();
        assert!((58..=60).contains(&retry_after));
    }
}
