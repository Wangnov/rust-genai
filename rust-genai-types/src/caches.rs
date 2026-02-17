use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::http::{HttpOptions, HttpResponse};
use crate::tool::{Tool, ToolConfig};

/// 创建缓存配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateCachedContentConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// Optional. TTL (e.g. "3600s").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
    /// Optional. Expire time (RFC3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<String>,
    /// Optional. Display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Optional. Contents to cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<Content>>,
    /// Optional. System instruction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    /// Optional. Tools list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Optional. Tool config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<ToolConfig>,
    /// Optional. KMS key name (Vertex only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key_name: Option<String>,
}

/// 更新缓存配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCachedContentConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// Optional. TTL (e.g. "3600s").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
    /// Optional. Expire time (RFC3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<String>,
}

/// 获取缓存配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetCachedContentConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// 删除缓存配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCachedContentConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// 删除缓存响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCachedContentResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
}

/// 列出缓存配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListCachedContentsConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// Optional. Page size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    /// Optional. Page token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

/// 缓存使用元数据。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CachedContentUsageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_duration_seconds: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_token_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_duration_seconds: Option<i32>,
}

/// 缓存内容。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CachedContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<CachedContentUsageMetadata>,
}

/// 列表响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListCachedContentsResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_contents: Option<Vec<CachedContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}
