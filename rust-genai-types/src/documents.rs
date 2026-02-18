use serde::{Deserialize, Serialize};

use crate::enums::DocumentState;
use crate::http::{HttpOptions, HttpResponse};

/// User provided string values assigned to a single metadata key.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StringList {
    #[serde(default)]
    pub values: Vec<String>,
}

/// User provided metadata stored as key-value pairs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustomMetadata {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numeric_value: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_list_value: Option<StringList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
}

/// A Document is a collection of chunks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<DocumentState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_metadata: Option<Vec<CustomMetadata>>,
}

/// Optional parameters for getting a Document.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetDocumentConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// Optional parameters for deleting a Document.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteDocumentConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    /// Optional. Force delete related chunks and objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
}

/// Optional parameters for listing Documents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListDocumentsConfig {
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

/// Response for listing Documents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListDocumentsResponse {
    /// Optional. Used to retain the full HTTP response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk_http_response: Option<HttpResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documents: Option<Vec<Document>>,
}
