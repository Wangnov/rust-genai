use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::enums::JobState;
use crate::http::{HttpOptions, HttpResponse};
use crate::models::{EmbedContentConfig, GenerateContentConfig};
use crate::response::GenerateContentResponse;

/// 内联请求。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InlinedRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<Content>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<GenerateContentConfig>,
}

/// 批处理输入源。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcs_uri: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bigquery_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlined_requests: Option<Vec<InlinedRequest>>,
}

/// 批处理错误信息。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JobError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 内联响应（生成内容）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InlinedResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<GenerateContentResponse>,
    /// Optional. The metadata to be associated with the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JobError>,
}

/// 单条嵌入响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SingleEmbedContentResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<crate::models::ContentEmbedding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<String>,
}

/// 内联嵌入响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InlinedEmbedContentResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<SingleEmbedContentResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JobError>,
}

/// 批处理输出目标。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatchJobDestination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcs_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bigquery_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlined_responses: Option<Vec<InlinedResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlined_embed_content_responses: Option<Vec<InlinedEmbedContentResponse>>,
}

/// 任务完成统计（Vertex）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompletionStats {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub successful_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub successful_forecast_point_count: Option<String>,
}

/// 批处理任务。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BatchJob {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<JobState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JobError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<BatchJobSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<BatchJobDestination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_stats: Option<CompletionStats>,
}

/// 创建批处理任务配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateBatchJobConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest: Option<BatchJobDestination>,
}

/// Embeddings batch 请求体（仅用于内联）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmbedContentBatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<Vec<Content>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<EmbedContentConfig>,
}

/// Embeddings batch 输入源（Gemini inline）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingsBatchJobSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlined_requests: Option<EmbedContentBatch>,
}

/// 获取批处理任务配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetBatchJobConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// 删除批处理任务配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteBatchJobConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// 列出批处理任务配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListBatchJobsConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

/// 批处理列表响应。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListBatchJobsResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_jobs: Option<Vec<BatchJob>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}
