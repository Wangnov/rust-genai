use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::enums::{AdapterSize, JobState, TuningMethod};
use crate::http::HttpOptions;

/// Tuning job state alias.
pub type TuningJobState = JobState;

/// A single example for tuning (Gemini API only).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TuningExample {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_input: Option<String>,
}

/// Supervised fine-tuning training dataset.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TuningDataset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcs_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_dataset_resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<TuningExample>>,
}

/// Validation dataset for tuning.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TuningValidationDataset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcs_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_dataset_resource: Option<String>,
}

/// Evaluation config (pass-through structure).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autorater_config: Option<Value>,
}

/// Fine-tuning job creation request - optional fields.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateTuningJobConfig {
    /// Optional. HTTP request overrides (SDK only, not sent to API).
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<TuningMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_dataset: Option<TuningValidationDataset>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuned_model_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_last_checkpoint_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_tuned_model_checkpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_size: Option<AdapterSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation_config: Option<EvaluationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beta: Option<f32>,
}

/// Configuration for the list tuning jobs method.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListTuningJobsConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

/// Optional parameters for tunings.get method.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetTuningJobConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// Optional parameters for tunings.cancel method.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CancelTuningJobConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub http_options: Option<HttpOptions>,
}

/// `TunedModel` checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TunedModelCheckpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

/// `TunedModel` for the tuning job result.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TunedModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoints: Option<Vec<TunedModelCheckpoint>>,
}

/// Pre-tuned model for continuous tuning (Vertex AI).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PreTunedModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuned_model_name: Option<String>,
}

/// google.rpc.Status compatible error.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GoogleRpcStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// A tuning job.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TuningJob {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<TuningJobState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<GoogleRpcStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuned_model: Option<TunedModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_tuned_model: Option<PreTunedModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supervised_tuning_spec: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preference_optimization_spec: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuning_data_stats: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_spec: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_model_tuning_spec: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation_config: Option<EvaluationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_base_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experiment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_job: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuned_model_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub veo_tuning_spec: Option<Value>,
}

/// Response for list tuning jobs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListTuningJobsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tuning_jobs: Option<Vec<TuningJob>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Tuning operation (Gemini Developer API).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TuningOperation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}
