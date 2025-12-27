use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::config::SafetySetting;
use crate::config::{GenerationConfig, SpeechConfig};
use crate::http::HttpOptions;
use crate::tool::{Tool, ToolConfig};

/// Interactions 输入（支持文本、内容列表或完整 turns）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InteractionInput {
    Text(String),
    Contents(Vec<InteractionContent>),
    Turns(Vec<InteractionTurn>),
}

impl InteractionInput {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    pub fn contents(contents: Vec<InteractionContent>) -> Self {
        Self::Contents(contents)
    }

    pub fn turns(turns: Vec<InteractionTurn>) -> Self {
        Self::Turns(turns)
    }
}

impl From<String> for InteractionInput {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for InteractionInput {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

/// Interactions 内容（支持多种 type，未知字段保留到 extra）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InteractionContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl InteractionContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content_type: "text".into(),
            text: Some(text.into()),
            ..Default::default()
        }
    }

    pub fn image_data(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            content_type: "image".into(),
            data: Some(data.into()),
            mime_type: Some(mime_type.into()),
            ..Default::default()
        }
    }

    pub fn audio_data(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            content_type: "audio".into(),
            data: Some(data.into()),
            mime_type: Some(mime_type.into()),
            ..Default::default()
        }
    }
}

/// Interactions Turn。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InteractionTurn {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<InteractionContent>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Interactions 生成响应模态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractionResponseModality {
    Text,
    Image,
    Audio,
    Video,
    Document,
}

/// Thinking level（Interactions 版本，lowercase）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractionThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
}

/// Thinking summaries 输出策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractionThinkingSummaries {
    Auto,
    #[serde(rename = "none")]
    NoneValue,
}

/// AgentConfig（可选）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InteractionAgentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<InteractionInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<InteractionResponseModality>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<InteractionThinkingLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_summaries: Option<InteractionThinkingSummaries>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech_config: Option<SpeechConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Create Interaction 配置（请求体）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateInteractionConfig {
    /// Optional. HTTP request overrides (SDK only).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    pub model: String,
    pub input: InteractionInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<InteractionInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<InteractionResponseModality>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<InteractionThinkingLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_summaries: Option<InteractionThinkingSummaries>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_concurrent_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_config: Option<InteractionAgentConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
}

impl CreateInteractionConfig {
    pub fn new(model: impl Into<String>, input: impl Into<InteractionInput>) -> Self {
        Self {
            http_options: None,
            model: model.into(),
            input: input.into(),
            tools: None,
            tool_choice: None,
            tool_config: None,
            generation_config: None,
            safety_settings: None,
            system_instruction: None,
            response_modalities: None,
            thinking_level: None,
            thinking_summaries: None,
            allow_concurrent_tool_calls: None,
            agent_config: None,
            background: None,
            store: None,
            labels: None,
        }
    }
}

/// Get Interaction 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct GetInteractionConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// Delete Interaction 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DeleteInteractionConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// Cancel Interaction 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct CancelInteractionConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// Interaction usage by modality.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InteractionUsageBucket {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<i32>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Interaction usage summary.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InteractionUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_token_details: Option<InteractionUsageBucket>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_token_details: Option<InteractionUsageBucket>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Interaction resource.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct Interaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<InteractionInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<InteractionContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<InteractionUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// SSE 事件载荷。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InteractionEvent {
    #[serde(rename = "event_type", alias = "eventType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Interaction>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_text_serializes_as_string() {
        let input = InteractionInput::text("hello");
        let json = serde_json::to_string(&input).unwrap();
        assert_eq!(json, "\"hello\"");
    }

    #[test]
    fn input_contents_serializes_as_array() {
        let input = InteractionInput::contents(vec![InteractionContent::text("hi")]);
        let value = serde_json::to_value(&input).unwrap();
        assert!(value.is_array());
    }

    #[test]
    fn interaction_event_roundtrip() {
        let json = r#"{
            "event_type": "interactions.create",
            "data": {
                "id": "int_123",
                "model": "gemini-2.0-flash",
                "outputs": [{"type":"text","text":"ok"}]
            }
        }"#;
        let event: InteractionEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type.as_deref(), Some("interactions.create"));
        assert_eq!(
            event.data.as_ref().and_then(|d| d.id.as_deref()),
            Some("int_123")
        );
    }
}
