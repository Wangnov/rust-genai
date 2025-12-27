//! Converter module.

use serde_json::Value;

use crate::models::{
    ComputeTokensRequest, ComputeTokensResponse, CountTokensRequest, CountTokensResponse,
    GenerateContentRequest,
};
use crate::response::GenerateContentResponse;

/// GenerateContent 请求转换（Gemini API）。
pub fn generate_content_request_to_mldev(
    request: &GenerateContentRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// GenerateContent 请求转换（Vertex AI）。
pub fn generate_content_request_to_vertex(
    request: &GenerateContentRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// GenerateContent 响应转换（Gemini API）。
pub fn generate_content_response_from_mldev(
    value: Value,
) -> Result<GenerateContentResponse, serde_json::Error> {
    match serde_json::from_value(value.clone()) {
        Ok(response) => Ok(response),
        Err(err) => {
            if std::env::var("RUST_GENAI_DEBUG_RESPONSE").is_ok() {
                eprintln!("GenerateContentResponse parse failed: {err}");
                eprintln!("Raw response: {}", value);
            }
            Err(err)
        }
    }
}

/// GenerateContent 响应转换（Vertex AI）。
pub fn generate_content_response_from_vertex(
    value: Value,
) -> Result<GenerateContentResponse, serde_json::Error> {
    match serde_json::from_value(value.clone()) {
        Ok(response) => Ok(response),
        Err(err) => {
            if std::env::var("RUST_GENAI_DEBUG_RESPONSE").is_ok() {
                eprintln!("GenerateContentResponse parse failed: {err}");
                eprintln!("Raw response: {}", value);
            }
            Err(err)
        }
    }
}

/// CountTokens 请求转换（Gemini API）。
pub fn count_tokens_request_to_mldev(
    request: &CountTokensRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// CountTokens 请求转换（Vertex AI）。
pub fn count_tokens_request_to_vertex(
    request: &CountTokensRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// CountTokens 响应转换（Gemini API）。
pub fn count_tokens_response_from_mldev(
    value: Value,
) -> Result<CountTokensResponse, serde_json::Error> {
    serde_json::from_value(value)
}

/// CountTokens 响应转换（Vertex AI）。
pub fn count_tokens_response_from_vertex(
    value: Value,
) -> Result<CountTokensResponse, serde_json::Error> {
    serde_json::from_value(value)
}

/// ComputeTokens 请求转换（Vertex AI）。
pub fn compute_tokens_request_to_vertex(
    request: &ComputeTokensRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// ComputeTokens 响应转换（Vertex AI）。
pub fn compute_tokens_response_from_vertex(
    value: Value,
) -> Result<ComputeTokensResponse, serde_json::Error> {
    serde_json::from_value(value)
}
