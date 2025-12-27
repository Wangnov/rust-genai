//! Error definitions for the SDK.

use thiserror::Error;

#[cfg(feature = "mcp")]
use rmcp::service::ServiceError;

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

pub type Result<T> = std::result::Result<T, Error>;
