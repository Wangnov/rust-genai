use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::http::HttpOptions;

/// Interactions 输入（支持文本、内容列表或完整 turns）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InteractionInput {
    Text(String),
    Content(InteractionContent),
    Contents(Vec<InteractionContent>),
    Turns(Vec<InteractionTurn>),
}

impl InteractionInput {
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    #[must_use]
    pub const fn content(content: InteractionContent) -> Self {
        Self::Content(content)
    }

    #[must_use]
    pub const fn contents(contents: Vec<InteractionContent>) -> Self {
        Self::Contents(contents)
    }

    #[must_use]
    pub const fn turns(turns: Vec<InteractionTurn>) -> Self {
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
    pub summary: Option<Value>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InteractionTurn {
    pub role: String,
    pub content: InteractionTurnContent,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Turn 的 content 字段：可以是 string 或 content 数组。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InteractionTurnContent {
    Text(String),
    Contents(Vec<InteractionContent>),
}

impl InteractionTurnContent {
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    #[must_use]
    pub const fn contents(contents: Vec<InteractionContent>) -> Self {
        Self::Contents(contents)
    }
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

/// Tool choice mode（Interactions）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceType {
    Auto,
    Any,
    #[serde(rename = "none")]
    NoneValue,
    Validated,
}

/// The configuration for allowed tools.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct AllowedTools {
    /// The mode of the tool choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ToolChoiceType>,
    /// The names of the allowed tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Tool choice config wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ToolChoiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<AllowedTools>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// ToolChoice: a string mode or a config object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Type(ToolChoiceType),
    Config(ToolChoiceConfig),
}

/// Computer-use tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputerUseTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(rename = "excludedPredefinedFunctions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_predefined_functions: Option<Vec<String>>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// MCP server tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct McpServerTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<AllowedTools>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// File search tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct FileSearchTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_search_store_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Function tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FunctionTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// JSON schema for the function parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Interactions tool declarations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Tool {
    #[serde(rename = "function")]
    Function(FunctionTool),
    #[serde(rename = "google_search")]
    GoogleSearch,
    #[serde(rename = "code_execution")]
    CodeExecution,
    #[serde(rename = "url_context")]
    UrlContext,
    #[serde(rename = "computer_use")]
    ComputerUse(ComputerUseTool),
    #[serde(rename = "mcp_server")]
    McpServer(McpServerTool),
    #[serde(rename = "file_search")]
    FileSearch(FileSearchTool),
}

/// The configuration for speech interaction.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct SpeechConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// The configuration for image interaction.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ImageConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_size: Option<String>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Configuration parameters for model interactions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<InteractionThinkingLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_summaries: Option<InteractionThinkingSummaries>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech_config: Option<Vec<SpeechConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_config: Option<ImageConfig>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Configuration for dynamic agents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DynamicAgentConfig {
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Configuration for the Deep Research agent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DeepResearchAgentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_summaries: Option<InteractionThinkingSummaries>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Agent configuration union (discriminator: `type`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentConfig {
    #[serde(rename = "dynamic")]
    Dynamic(DynamicAgentConfig),
    #[serde(rename = "deep-research")]
    DeepResearch(DeepResearchAgentConfig),
}

/// Create Interaction 配置（请求体）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateInteractionConfig {
    /// Optional. HTTP request overrides (SDK only).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// The name of the `Model` used for generating the interaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The name of the `Agent` used for generating the interaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    pub input: InteractionInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_config: Option<AgentConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_interaction_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<InteractionResponseModality>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl CreateInteractionConfig {
    pub fn new(model: impl Into<String>, input: impl Into<InteractionInput>) -> Self {
        Self {
            http_options: None,
            model: Some(model.into()),
            agent: None,
            input: input.into(),
            tools: None,
            generation_config: None,
            agent_config: None,
            background: None,
            store: None,
            previous_interaction_id: None,
            response_format: None,
            response_mime_type: None,
            response_modalities: None,
            system_instruction: None,
            stream: None,
        }
    }

    pub fn new_agent(agent: impl Into<String>, input: impl Into<InteractionInput>) -> Self {
        Self {
            http_options: None,
            model: None,
            agent: Some(agent.into()),
            input: input.into(),
            tools: None,
            generation_config: None,
            agent_config: None,
            background: None,
            store: None,
            previous_interaction_id: None,
            response_format: None,
            response_mime_type: None,
            response_modalities: None,
            system_instruction: None,
            stream: None,
        }
    }
}

/// Get Interaction 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct GetInteractionConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// If set to true, includes the input in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_input: Option<bool>,
    /// If set to true, the generated content will be streamed incrementally.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Optional. Resumes the interaction stream from the next chunk after the event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_id: Option<String>,
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
    pub total_input_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_thought_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cached_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tool_use_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens_by_modality: Option<Vec<InteractionTokensByModality>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Token usage by modality.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InteractionTokensByModality {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<i32>,
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
    pub previous_interaction_id: Option<String>,
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
pub struct InteractionSseEvent {
    #[serde(rename = "event_type", alias = "eventType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(rename = "event_id", alias = "eventId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// The Interaction resource (present for start/complete events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction: Option<Interaction>,
    /// Present for interaction.status_update events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_id: Option<String>,
    /// Present for interaction.status_update events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Present for content.* events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
    /// Present for content.start events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<InteractionContent>,
    /// Present for content.delta events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<InteractionContent>,
    /// Present for error events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<InteractionError>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// Backward-compatible alias.
#[deprecated(note = "Renamed to InteractionSseEvent")]
pub type InteractionEvent = InteractionSseEvent;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InteractionError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
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
    fn interaction_event_start_roundtrip() {
        let json = r#"{
            "event_type": "interaction.start",
            "event_id": "evt_1",
            "interaction": {
                "id": "int_123",
                "status": "in_progress"
            }
        }"#;
        let event: InteractionSseEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type.as_deref(), Some("interaction.start"));
        assert_eq!(event.event_id.as_deref(), Some("evt_1"));
        assert_eq!(
            event.interaction.as_ref().and_then(|d| d.id.as_deref()),
            Some("int_123")
        );
    }

    #[test]
    fn interaction_content_helpers() {
        let image = InteractionContent::image_data("AAAA", "image/png");
        assert_eq!(image.content_type, "image");
        assert_eq!(image.mime_type.as_deref(), Some("image/png"));

        let audio = InteractionContent::audio_data("BBBB", "audio/wav");
        assert_eq!(audio.content_type, "audio");
        assert_eq!(audio.mime_type.as_deref(), Some("audio/wav"));
    }

    #[test]
    fn interaction_input_from_str() {
        let input: InteractionInput = "hi".into();
        let json = serde_json::to_string(&input).unwrap();
        assert_eq!(json, "\"hi\"");
    }

    #[test]
    fn create_interaction_config_new_sets_fields() {
        let config = CreateInteractionConfig::new("model-1", "hello");
        assert_eq!(config.model.as_deref(), Some("model-1"));
        match config.input {
            InteractionInput::Text(value) => assert_eq!(value, "hello"),
            _ => panic!("expected text input"),
        }
    }

    #[test]
    fn interaction_event_alias_event_type() {
        let json = r#"{
            "eventType": "interaction.complete",
            "eventId": "evt_9",
            "interaction": { "id": "int_456", "status": "completed" }
        }"#;
        let event: InteractionSseEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type.as_deref(), Some("interaction.complete"));
        assert_eq!(event.event_id.as_deref(), Some("evt_9"));
        assert_eq!(
            event.interaction.as_ref().and_then(|d| d.id.as_deref()),
            Some("int_456")
        );
    }
}
