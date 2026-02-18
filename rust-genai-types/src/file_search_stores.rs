use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::documents::CustomMetadata;
use crate::http::{HttpOptions, HttpResponse};
use crate::operations::OperationError;

/// Optional parameters for creating a file search store.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateFileSearchStoreConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// Optional. Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// A collection of Documents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileSearchStore {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_documents_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_documents_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_documents_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<String>,
}

/// Optional parameters for getting a file search store.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetFileSearchStoreConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// Optional parameters for deleting a file search store.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFileSearchStoreConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// Optional. Force delete related documents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
}

/// Optional parameters for listing file search stores.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListFileSearchStoresConfig {
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

/// Response for listing file search stores.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListFileSearchStoresResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_search_stores: Option<Vec<FileSearchStore>>,
}

/// Configuration for a white space chunking algorithm.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WhiteSpaceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_chunk: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_overlap_tokens: Option<i32>,
}

/// Config for telling the service how to chunk the file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChunkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub white_space_config: Option<WhiteSpaceConfig>,
}

/// Optional parameters for uploading a file to a `FileSearchStore`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UploadToFileSearchStoreConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// Optional. If true, returns the raw HTTP response body in `sdk_http_response.body` (SDK only).
    #[serde(skip_serializing, skip_deserializing)]
    pub should_return_http_response: Option<bool>,
    /// Optional. MIME type of the file to be uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Optional. Display name of the created document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Optional. User provided custom metadata stored as key-value pairs used for querying.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_metadata: Option<Vec<CustomMetadata>>,
    /// Optional. Config for telling the service how to chunk the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_config: Option<ChunkingConfig>,
}

/// Optional parameters for importing a file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportFileConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// Optional. User provided custom metadata stored as key-value pairs used for querying.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_metadata: Option<Vec<CustomMetadata>>,
    /// Optional. Config for telling the service how to chunk the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_config: Option<ChunkingConfig>,
}

/// Response for importing a file into a `FileSearchStore`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportFileResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_name: Option<String>,
}

/// Response for the resumable upload method.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UploadToFileSearchStoreResumableResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
}

/// Response for uploading a file into a `FileSearchStore`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UploadToFileSearchStoreResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_name: Option<String>,
}

/// Long-running operation for importing a file to a `FileSearchStore`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportFileOperation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ImportFileResponse>,
}

/// Long-running operation for uploading a file to a `FileSearchStore`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UploadToFileSearchStoreOperation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<UploadToFileSearchStoreResponse>,
}
