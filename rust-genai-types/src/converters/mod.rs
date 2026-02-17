//! Converter module.

use serde_json::Value;

use crate::models::{
    ComputeTokensRequest, ComputeTokensResponse, CountTokensRequest, CountTokensResponse,
    GenerateContentRequest,
};
use crate::response::GenerateContentResponse;

/// `GenerateContent` 请求转换（Gemini API）。
///
/// # Errors
/// 当序列化失败时返回错误。
pub fn generate_content_request_to_mldev(
    request: &GenerateContentRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// `GenerateContent` 请求转换（Vertex AI）。
///
/// # Errors
/// 当序列化失败时返回错误。
pub fn generate_content_request_to_vertex(
    request: &GenerateContentRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// `GenerateContent` 响应转换（Gemini API）。
///
/// # Errors
/// 当反序列化失败时返回错误。
pub fn generate_content_response_from_mldev(
    value: Value,
) -> Result<GenerateContentResponse, serde_json::Error> {
    let raw = std::env::var("RUST_GENAI_DEBUG_RESPONSE")
        .is_ok()
        .then(|| value.clone());
    match serde_json::from_value(value) {
        Ok(response) => Ok(response),
        Err(err) => {
            if let Some(raw) = raw {
                eprintln!("GenerateContentResponse parse failed: {err}");
                eprintln!("Raw response: {raw}");
            }
            Err(err)
        }
    }
}

/// `GenerateContent` 响应转换（Vertex AI）。
///
/// # Errors
/// 当反序列化失败时返回错误。
pub fn generate_content_response_from_vertex(
    value: Value,
) -> Result<GenerateContentResponse, serde_json::Error> {
    let raw = std::env::var("RUST_GENAI_DEBUG_RESPONSE")
        .is_ok()
        .then(|| value.clone());
    match serde_json::from_value(value) {
        Ok(response) => Ok(response),
        Err(err) => {
            if let Some(raw) = raw {
                eprintln!("GenerateContentResponse parse failed: {err}");
                eprintln!("Raw response: {raw}");
            }
            Err(err)
        }
    }
}

/// `CountTokens` 请求转换（Gemini API）。
///
/// # Errors
/// 当序列化失败时返回错误。
pub fn count_tokens_request_to_mldev(
    request: &CountTokensRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// `CountTokens` 请求转换（Vertex AI）。
///
/// # Errors
/// 当序列化失败时返回错误。
pub fn count_tokens_request_to_vertex(
    request: &CountTokensRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// `CountTokens` 响应转换（Gemini API）。
///
/// # Errors
/// 当反序列化失败时返回错误。
pub fn count_tokens_response_from_mldev(
    value: Value,
) -> Result<CountTokensResponse, serde_json::Error> {
    serde_json::from_value(value)
}

/// `CountTokens` 响应转换（Vertex AI）。
///
/// # Errors
/// 当反序列化失败时返回错误。
pub fn count_tokens_response_from_vertex(
    value: Value,
) -> Result<CountTokensResponse, serde_json::Error> {
    serde_json::from_value(value)
}

/// `ComputeTokens` 请求转换（Vertex AI）。
///
/// # Errors
/// 当序列化失败时返回错误。
pub fn compute_tokens_request_to_vertex(
    request: &ComputeTokensRequest,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(request)
}

/// `ComputeTokens` 响应转换（Vertex AI）。
///
/// # Errors
/// 当反序列化失败时返回错误。
pub fn compute_tokens_response_from_vertex(
    value: Value,
) -> Result<ComputeTokensResponse, serde_json::Error> {
    serde_json::from_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::models::{ComputeTokensRequest, CountTokensRequest, GenerateContentRequest};
    use serde_json::json;
    use std::env;

    #[test]
    fn generate_content_roundtrip_success() {
        let request = GenerateContentRequest {
            contents: vec![Content::text("hi")],
            system_instruction: None,
            safety_settings: None,
            tools: None,
            tool_config: None,
            generation_config: None,
            model_armor_config: None,
            cached_content: None,
            labels: None,
        };
        let value = generate_content_request_to_mldev(&request).unwrap();
        let response = generate_content_response_from_mldev(value).unwrap();
        assert_eq!(response.candidates.len(), 0);
    }

    #[test]
    fn generate_content_response_error_with_debug() {
        env::set_var("RUST_GENAI_DEBUG_RESPONSE", "1");
        let result = generate_content_response_from_vertex(Value::String("bad".into()));
        env::remove_var("RUST_GENAI_DEBUG_RESPONSE");
        assert!(result.is_err());
    }

    #[test]
    fn count_and_compute_tokens_conversions() {
        let count_request = CountTokensRequest {
            contents: vec![Content::text("count")],
            system_instruction: None,
            tools: None,
            generation_config: None,
        };
        let count_value = count_tokens_request_to_vertex(&count_request).unwrap();
        let count_response = count_tokens_response_from_vertex(json!({})).unwrap();
        assert!(count_value.is_object());
        assert_eq!(count_response.total_tokens, None);

        let compute_request = ComputeTokensRequest {
            contents: vec![Content::text("compute")],
        };
        let compute_value = compute_tokens_request_to_vertex(&compute_request).unwrap();
        let compute_response = compute_tokens_response_from_vertex(json!({})).unwrap();
        assert!(compute_value.is_object());
        assert!(compute_response.tokens_info.is_none());
    }
}
