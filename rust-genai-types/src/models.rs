use std::collections::HashMap;

use crate::base64_serde;
use crate::config::{GenerationConfig, ModelArmorConfig, SafetySetting};
use crate::content::Content;
use crate::enums::{
    ControlReferenceType, EditMode, ImagePromptLanguage, MaskReferenceMode, PersonGeneration,
    ReferenceImageType, SafetyFilterLevel, SegmentMode, SubjectReferenceType,
    VideoCompressionQuality, VideoGenerationMaskMode, VideoGenerationReferenceType,
};
use crate::http::{HttpOptions, HttpResponse};
use crate::tool::{Tool, ToolConfig};
use serde::{Deserialize, Serialize};

/// `GenerateContent` 请求配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    /// Settings for prompt and response sanitization using the Model Armor service.
    /// If supplied, `safety_settings` must not be supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_armor_config: Option<ModelArmorConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    /// 自动函数调用（SDK 内部行为，不会发送到后端）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatic_function_calling: Option<AutomaticFunctionCallingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    /// Optional. If true, returns the raw HTTP response body in `sdk_http_response.body` (SDK only).
    ///
    /// Note: Not supported in streaming methods.
    #[serde(skip_serializing, skip_deserializing)]
    pub should_return_http_response: Option<bool>,
}

/// `GenerateContent` 请求体。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<SafetySetting>>,
    /// Settings for prompt and response sanitization using the Model Armor service.
    /// If supplied, `safety_settings` must not be supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_armor_config: Option<ModelArmorConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
}

/// 自动函数调用配置（SDK 内部配置）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutomaticFunctionCallingConfig {
    /// 是否禁用自动函数调用。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,
    /// 最大远程调用次数（默认 10，<=0 会禁用）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_remote_calls: Option<i32>,
    /// 是否忽略自动函数调用历史。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_call_history: Option<bool>,
}

/// `CountTokens` 请求配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CountTokensConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

/// `CountTokens` 请求体。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountTokensRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

/// `CountTokens` 响应体。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountTokensResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_content_token_count: Option<i32>,
}

/// `ComputeTokens` 请求配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ComputeTokensConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// `ComputeTokens` 请求体。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeTokensRequest {
    pub contents: Vec<Content>,
}

/// Tokens info with a list of tokens and the corresponding list of token ids.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokensInfo {
    /// Optional fields for the role from the corresponding Content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// A list of token ids from the input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_ids: Option<Vec<i64>>,
    /// A list of tokens from the input (base64-encoded strings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Vec<String>>,
}

/// Response for computing tokens.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ComputeTokensResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    /// Lists of tokens info from the input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_info: Option<Vec<TokensInfo>>,
}

/// `EmbedContent` 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmbedContentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimensionality: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_truncate: Option<bool>,
}

/// 内容嵌入统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentEmbeddingStatistics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<f32>,
}

/// 内容嵌入结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentEmbedding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statistics: Option<ContentEmbeddingStatistics>,
}

/// `EmbedContent` 元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedContentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billable_character_count: Option<i32>,
}

/// `EmbedContent` 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedContentResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<Vec<ContentEmbedding>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<EmbedContentMetadata>,
}

/// 模型信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_token_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_token_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_actions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
}

/// `ListModels` 请求配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListModelsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_base: Option<bool>,
}

/// `ListModels` 响应体。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListModelsResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<Model>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// `UpdateModel` 请求配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateModelConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_checkpoint_id: Option<String>,
}

/// `DeleteModel` 请求配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteModelConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// `DeleteModel` 响应体（空响应）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteModelResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
}

/// 图像生成配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerateImagesConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_gcs_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_images: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_filter_level: Option<SafetyFilterLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_generation: Option<PersonGeneration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_safety_attributes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_rai_reason: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<ImagePromptLanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression_quality: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_watermark: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhance_prompt: Option<bool>,
}

/// 图像。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcs_uri: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "bytesBase64Encoded",
        with = "base64_serde::option"
    )]
    pub image_bytes: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Safety attributes for generated media.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SafetyAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scores: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// 生成图像输出。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedImage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rai_filtered_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_attributes: Option<SafetyAttributes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_prompt: Option<String>,
}

/// 图像生成响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerateImagesResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(default)]
    pub generated_images: Vec<GeneratedImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub positive_prompt_safety_attributes: Option<SafetyAttributes>,
}

/// Mask reference config for image editing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MaskReferenceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_mode: Option<MaskReferenceMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segmentation_classes: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_dilation: Option<f32>,
}

/// Control reference config for image editing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ControlReferenceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_type: Option<ControlReferenceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_control_image_computation: Option<bool>,
}

/// Style reference config for image editing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StyleReferenceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_description: Option<String>,
}

/// Subject reference config for image editing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SubjectReferenceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_type: Option<SubjectReferenceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_description: Option<String>,
}

/// Reference image for image editing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceImage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_type: Option<ReferenceImageType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_image_config: Option<MaskReferenceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub control_image_config: Option<ControlReferenceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_image_config: Option<StyleReferenceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_image_config: Option<SubjectReferenceConfig>,
}

/// 编辑图像配置（Vertex）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditImageConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_gcs_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_images: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_filter_level: Option<SafetyFilterLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_generation: Option<PersonGeneration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_safety_attributes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_rai_reason: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<ImagePromptLanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression_quality: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_watermark: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_mode: Option<EditMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_steps: Option<i32>,
}

/// 编辑图像响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EditImageResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(default)]
    pub generated_images: Vec<GeneratedImage>,
}

/// Upscale 图像配置（Vertex）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpscaleImageConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_gcs_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_filter_level: Option<SafetyFilterLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_generation: Option<PersonGeneration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_rai_reason: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression_quality: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhance_input_image: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_preservation_factor: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_images: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Upscale 图像响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpscaleImageResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(default)]
    pub generated_images: Vec<GeneratedImage>,
}

/// 产品图像（用于 Recontext）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProductImage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_image: Option<Image>,
}

/// Recontext 图像输入源。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecontextImageSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_images: Option<Vec<ProductImage>>,
}

/// Recontext 图像配置（Vertex）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecontextImageConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_images: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_steps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_gcs_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_filter_level: Option<SafetyFilterLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_generation: Option<PersonGeneration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_watermark: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression_quality: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhance_prompt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
}

/// Recontext 图像响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecontextImageResponse {
    #[serde(default)]
    pub generated_images: Vec<GeneratedImage>,
}

/// Scribble image（交互式分割）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScribbleImage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
}

/// Segment 图像输入源。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SegmentImageSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scribble_image: Option<ScribbleImage>,
}

/// Segment 图像配置（Vertex）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SegmentImageConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<SegmentMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_predictions: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_threshold: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_dilation: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_color_threshold: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
}

/// 分割实体标签。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EntityLabel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
}

/// 分割图像 mask 输出。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedImageMask {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<EntityLabel>>,
}

/// Segment 图像响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SegmentImageResponse {
    #[serde(default)]
    pub generated_masks: Vec<GeneratedImageMask>,
}

/// 视频。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "bytesBase64Encoded",
        with = "base64_serde::option"
    )]
    pub video_bytes: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// 视频生成输入源。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerateVideosSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<Video>,
}

/// 视频生成参考图像。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VideoGenerationReferenceImage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_type: Option<VideoGenerationReferenceType>,
}

/// 视频生成遮罩。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VideoGenerationMask {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_mode: Option<VideoGenerationMaskMode>,
}

/// 视频生成配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerateVideosConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_videos: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_gcs_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_generation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubsub_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhance_prompt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_audio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_frame: Option<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_images: Option<Vec<VideoGenerationReferenceImage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<VideoGenerationMask>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_quality: Option<VideoCompressionQuality>,
}

/// 生成视频输出。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedVideo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<Video>,
}

/// 视频生成响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerateVideosResponse {
    #[serde(default)]
    pub generated_videos: Vec<GeneratedVideo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rai_media_filtered_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rai_media_filtered_reasons: Option<Vec<String>>,
}
