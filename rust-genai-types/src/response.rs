use serde::{Deserialize, Serialize};

use crate::content::{Content, FunctionCall};
use crate::enums::{
    BlockedReason, FinishReason, HarmBlockThreshold, HarmCategory, HarmProbability, HarmSeverity,
    MediaModality, TrafficType, UrlRetrievalStatus,
};
use crate::grounding::{CitationMetadata, GroundingMetadata};
use crate::http::HttpResponse;
use crate::logprobs::LogprobsResult;

/// 生成内容响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatic_function_calling_history: Option<Vec<Content>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_feedback: Option<PromptFeedback>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<GenerateContentResponseUsageMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
}

impl GenerateContentResponse {
    /// 提取第一个候选的文本。
    #[must_use]
    pub fn text(&self) -> Option<String> {
        self.candidates
            .first()
            .and_then(|candidate| candidate.content.as_ref())
            .and_then(|content| content.first_text())
            .map(ToString::to_string)
    }

    /// 提取所有函数调用。
    #[must_use]
    pub fn function_calls(&self) -> Vec<&FunctionCall> {
        let mut calls = Vec::new();
        for candidate in &self.candidates {
            if let Some(content) = &candidate.content {
                for part in &content.parts {
                    if let Some(call) = part.function_call_ref() {
                        calls.push(call);
                    }
                }
            }
        }
        calls
    }
}

/// 响应候选。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation_metadata: Option<CitationMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_logprobs: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_metadata: Option<GroundingMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs_result: Option<LogprobsResult>,
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_context_metadata: Option<UrlContextMetadata>,
}

/// Prompt 反馈。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFeedback {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<BlockedReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason_message: Option<String>,
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
}

/// 安全评级。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetyRating {
    pub category: HarmCategory,
    pub probability: HarmProbability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwritten_threshold: Option<HarmBlockThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probability_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<HarmSeverity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity_score: Option<f64>,
}

/// 单一模态 token 统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModalityTokenCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality: Option<MediaModality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<i32>,
}

/// 生成请求/响应的用量统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_prompt_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thoughts_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<Vec<ModalityTokenCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_tokens_details: Option<Vec<ModalityTokenCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_tokens_details: Option<Vec<ModalityTokenCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_prompt_tokens_details: Option<Vec<ModalityTokenCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traffic_type: Option<TrafficType>,
}

/// `GenerateContentResponse` 使用的 usage metadata（包含 candidates 统计）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentResponseUsageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_tokens_details: Option<Vec<ModalityTokenCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates_tokens_details: Option<Vec<ModalityTokenCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<Vec<ModalityTokenCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thoughts_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_prompt_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_prompt_tokens_details: Option<Vec<ModalityTokenCount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traffic_type: Option<TrafficType>,
}

/// URL metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieved_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_retrieval_status: Option<UrlRetrievalStatus>,
}

/// URL Context 元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlContextMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_metadata: Option<Vec<UrlMetadata>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Role;
    use crate::content::{Content, FunctionCall, Part};
    use serde_json::json;

    #[test]
    fn response_text_and_function_calls() {
        let text_content = Content::from_parts(vec![Part::text("hello")], Role::Model);
        let call = FunctionCall {
            id: None,
            name: Some("lookup".into()),
            args: Some(json!({"q": "rust"})),
            partial_args: None,
            will_continue: None,
        };
        let call_content = Content::from_parts(vec![Part::function_call(call)], Role::Model);

        let response = GenerateContentResponse {
            sdk_http_response: None,
            candidates: vec![
                Candidate {
                    content: Some(text_content),
                    citation_metadata: None,
                    finish_message: None,
                    token_count: None,
                    finish_reason: None,
                    avg_logprobs: None,
                    grounding_metadata: None,
                    index: None,
                    logprobs_result: None,
                    safety_ratings: Vec::new(),
                    url_context_metadata: None,
                },
                Candidate {
                    content: Some(call_content),
                    citation_metadata: None,
                    finish_message: None,
                    token_count: None,
                    finish_reason: None,
                    avg_logprobs: None,
                    grounding_metadata: None,
                    index: None,
                    logprobs_result: None,
                    safety_ratings: Vec::new(),
                    url_context_metadata: None,
                },
            ],
            create_time: None,
            automatic_function_calling_history: None,
            prompt_feedback: None,
            usage_metadata: None,
            model_version: None,
            response_id: None,
        };

        assert_eq!(response.text(), Some("hello".to_string()));
        let calls = response.function_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name.as_deref(), Some("lookup"));
    }
}
