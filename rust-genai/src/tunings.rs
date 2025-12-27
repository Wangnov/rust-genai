//! Tunings API surface.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::enums::{JobState, TuningMethod};
use rust_genai_types::tunings::{
    CancelTuningJobConfig, CreateTuningJobConfig, GetTuningJobConfig, ListTuningJobsConfig,
    ListTuningJobsResponse, PreTunedModel, TunedModel, TuningDataset, TuningJob,
};
use serde_json::{json, Map, Value};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct Tunings {
    pub(crate) inner: Arc<ClientInner>,
}

impl Tunings {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建调优任务（默认配置）。
    pub async fn tune(
        &self,
        base_model: impl Into<String>,
        training_dataset: TuningDataset,
    ) -> Result<TuningJob> {
        self.tune_with_config(
            base_model,
            training_dataset,
            CreateTuningJobConfig::default(),
        )
        .await
    }

    /// 创建调优任务（带配置）。
    pub async fn tune_with_config(
        &self,
        base_model: impl Into<String>,
        training_dataset: TuningDataset,
        mut config: CreateTuningJobConfig,
    ) -> Result<TuningJob> {
        let http_options = config.http_options.take();
        let base_model = base_model.into();

        let body = match self.inner.config.backend {
            Backend::GeminiApi => {
                validate_mldev_config(&config)?;
                build_tune_body_mldev(&self.inner, &base_model, training_dataset, &config)?
            }
            Backend::VertexAi => {
                build_tune_body_vertex(&self.inner, &base_model, training_dataset, &config)?
            }
        };

        let mut body = body;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }

        let url = build_tuning_jobs_url(&self.inner, http_options.as_ref())?;
        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let value = response.json::<Value>().await?;
        match self.inner.config.backend {
            Backend::GeminiApi => parse_tuning_job_from_mldev(value),
            Backend::VertexAi => Ok(serde_json::from_value(value)?),
        }
    }

    /// 获取调优任务。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<TuningJob> {
        self.get_with_config(name, GetTuningJobConfig::default())
            .await
    }

    /// 获取调优任务（带配置）。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetTuningJobConfig,
    ) -> Result<TuningJob> {
        let http_options = config.http_options.take();
        let name = normalize_tuning_job_name(&self.inner, name.as_ref())?;
        let url = build_tuning_job_url(&self.inner, &name, http_options.as_ref())?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let value = response.json::<Value>().await?;
        match self.inner.config.backend {
            Backend::GeminiApi => parse_tuning_job_from_mldev(value),
            Backend::VertexAi => Ok(serde_json::from_value(value)?),
        }
    }

    /// 列出调优任务。
    pub async fn list(&self) -> Result<ListTuningJobsResponse> {
        self.list_with_config(ListTuningJobsConfig::default()).await
    }

    /// 列出调优任务（带配置）。
    pub async fn list_with_config(
        &self,
        mut config: ListTuningJobsConfig,
    ) -> Result<ListTuningJobsResponse> {
        let http_options = config.http_options.take();
        let url = build_tuning_jobs_url(&self.inner, http_options.as_ref())?;
        let url = add_list_query_params(url, &config)?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let value = response.json::<Value>().await?;
        match self.inner.config.backend {
            Backend::GeminiApi => parse_list_tuning_jobs_from_mldev(value),
            Backend::VertexAi => Ok(serde_json::from_value(value)?),
        }
    }

    /// 列出所有调优任务（自动翻页）。
    pub async fn all(&self) -> Result<Vec<TuningJob>> {
        self.all_with_config(ListTuningJobsConfig::default()).await
    }

    /// 列出所有调优任务（带配置，自动翻页）。
    pub async fn all_with_config(
        &self,
        mut config: ListTuningJobsConfig,
    ) -> Result<Vec<TuningJob>> {
        let mut jobs = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options = http_options.clone();
            let response = self.list_with_config(page_config).await?;
            if let Some(items) = response.tuning_jobs {
                jobs.extend(items);
            }
            match response.next_page_token {
                Some(token) if !token.is_empty() => {
                    config.page_token = Some(token);
                }
                _ => break,
            }
        }
        Ok(jobs)
    }

    /// 取消调优任务。
    pub async fn cancel(&self, name: impl AsRef<str>) -> Result<()> {
        self.cancel_with_config(name, CancelTuningJobConfig::default())
            .await
    }

    /// 取消调优任务（带配置）。
    pub async fn cancel_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: CancelTuningJobConfig,
    ) -> Result<()> {
        let http_options = config.http_options.take();
        let name = normalize_tuning_job_name(&self.inner, name.as_ref())?;
        let url = build_tuning_job_cancel_url(&self.inner, &name, http_options.as_ref())?;
        let mut request = self.inner.http.post(url).json(&json!({}));
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(())
    }
}

fn validate_mldev_config(config: &CreateTuningJobConfig) -> Result<()> {
    if config.validation_dataset.is_some() {
        return Err(Error::InvalidConfig {
            message: "validation_dataset is not supported in Gemini API".into(),
        });
    }
    if config.description.is_some() {
        return Err(Error::InvalidConfig {
            message: "description is not supported in Gemini API".into(),
        });
    }
    if config.export_last_checkpoint_only.is_some() {
        return Err(Error::InvalidConfig {
            message: "export_last_checkpoint_only is not supported in Gemini API".into(),
        });
    }
    if config.pre_tuned_model_checkpoint_id.is_some() {
        return Err(Error::InvalidConfig {
            message: "pre_tuned_model_checkpoint_id is not supported in Gemini API".into(),
        });
    }
    if config.adapter_size.is_some() {
        return Err(Error::InvalidConfig {
            message: "adapter_size is not supported in Gemini API".into(),
        });
    }
    if config.evaluation_config.is_some() {
        return Err(Error::InvalidConfig {
            message: "evaluation_config is not supported in Gemini API".into(),
        });
    }
    if config.labels.is_some() {
        return Err(Error::InvalidConfig {
            message: "labels is not supported in Gemini API".into(),
        });
    }
    if config.beta.is_some() {
        return Err(Error::InvalidConfig {
            message: "beta is not supported in Gemini API".into(),
        });
    }
    Ok(())
}

fn build_tune_body_mldev(
    inner: &ClientInner,
    base_model: &str,
    training_dataset: TuningDataset,
    config: &CreateTuningJobConfig,
) -> Result<Value> {
    if training_dataset.gcs_uri.is_some() {
        return Err(Error::InvalidConfig {
            message: "gcs_uri is not supported in Gemini API".into(),
        });
    }
    if training_dataset.vertex_dataset_resource.is_some() {
        return Err(Error::InvalidConfig {
            message: "vertex_dataset_resource is not supported in Gemini API".into(),
        });
    }

    let mut body = Map::new();
    let base_model = transform_model_name(inner.config.backend, base_model);
    body.insert("baseModel".to_string(), Value::String(base_model));

    if let Some(display_name) = &config.tuned_model_display_name {
        body.insert(
            "displayName".to_string(),
            Value::String(display_name.clone()),
        );
    }

    if let Some(examples) = training_dataset.examples {
        let examples_value = serde_json::to_value(examples)?;
        body.insert(
            "examples".to_string(),
            json!({ "examples": examples_value }),
        );
    }

    let mut hyper = Map::new();
    if let Some(epoch_count) = config.epoch_count {
        hyper.insert("epochCount".to_string(), Value::Number(epoch_count.into()));
    }
    if let Some(batch_size) = config.batch_size {
        hyper.insert("batchSize".to_string(), Value::Number(batch_size.into()));
    }
    if let Some(learning_rate) = config.learning_rate {
        if let Some(value) = serde_json::Number::from_f64(learning_rate as f64) {
            hyper.insert("learningRate".to_string(), Value::Number(value));
        }
    }
    if let Some(learning_rate_multiplier) = config.learning_rate_multiplier {
        if let Some(value) = serde_json::Number::from_f64(learning_rate_multiplier as f64) {
            hyper.insert("learningRateMultiplier".to_string(), Value::Number(value));
        }
    }

    if !hyper.is_empty() {
        body.insert(
            "tuningTask".to_string(),
            json!({ "hyperparameters": Value::Object(hyper) }),
        );
    }

    Ok(Value::Object(body))
}

fn build_tune_body_vertex(
    inner: &ClientInner,
    base_model: &str,
    training_dataset: TuningDataset,
    config: &CreateTuningJobConfig,
) -> Result<Value> {
    if config.batch_size.is_some() {
        return Err(Error::InvalidConfig {
            message: "batch_size is not supported in Vertex AI".into(),
        });
    }
    if config.learning_rate.is_some() {
        return Err(Error::InvalidConfig {
            message: "learning_rate is not supported in Vertex AI".into(),
        });
    }
    if training_dataset.examples.is_some() {
        return Err(Error::InvalidConfig {
            message: "examples is not supported in Vertex AI".into(),
        });
    }

    let method = config.method.unwrap_or(TuningMethod::SupervisedFineTuning);
    let spec_key = match method {
        TuningMethod::SupervisedFineTuning => "supervisedTuningSpec",
        TuningMethod::PreferenceTuning => "preferenceOptimizationSpec",
    };

    let mut body = Map::new();
    let mut spec = Map::new();

    if base_model.starts_with("projects/") {
        let mut pre_tuned_model = PreTunedModel {
            tuned_model_name: Some(base_model.to_string()),
            checkpoint_id: config.pre_tuned_model_checkpoint_id.clone(),
            base_model: None,
        };
        if pre_tuned_model.checkpoint_id.is_none() {
            pre_tuned_model.checkpoint_id = None;
        }
        body.insert(
            "preTunedModel".to_string(),
            serde_json::to_value(pre_tuned_model)?,
        );
    } else {
        let base_model = transform_model_name(inner.config.backend, base_model);
        body.insert("baseModel".to_string(), Value::String(base_model));
    }

    if let Some(uri) = training_dataset
        .gcs_uri
        .or(training_dataset.vertex_dataset_resource)
    {
        spec.insert("trainingDatasetUri".to_string(), Value::String(uri));
    }

    if let Some(validation_dataset) = &config.validation_dataset {
        let validation_uri = validation_dataset
            .gcs_uri
            .clone()
            .or(validation_dataset.vertex_dataset_resource.clone());
        if let Some(uri) = validation_uri {
            spec.insert("validationDatasetUri".to_string(), Value::String(uri));
        }
    }

    if let Some(display_name) = &config.tuned_model_display_name {
        body.insert(
            "tunedModelDisplayName".to_string(),
            Value::String(display_name.clone()),
        );
    }
    if let Some(description) = &config.description {
        body.insert(
            "description".to_string(),
            Value::String(description.clone()),
        );
    }

    let mut hyper = Map::new();
    if let Some(epoch_count) = config.epoch_count {
        hyper.insert("epochCount".to_string(), Value::Number(epoch_count.into()));
    }
    if let Some(learning_rate_multiplier) = config.learning_rate_multiplier {
        if let Some(value) = serde_json::Number::from_f64(learning_rate_multiplier as f64) {
            hyper.insert("learningRateMultiplier".to_string(), Value::Number(value));
        }
    }
    if let Some(adapter_size) = config.adapter_size {
        hyper.insert(
            "adapterSize".to_string(),
            serde_json::to_value(adapter_size)?,
        );
    }
    if let Some(beta) = config.beta {
        if let Some(value) = serde_json::Number::from_f64(beta as f64) {
            if matches!(method, TuningMethod::PreferenceTuning) {
                hyper.insert("beta".to_string(), Value::Number(value));
            } else {
                let mut preference_spec = Map::new();
                let mut pref_hyper = Map::new();
                pref_hyper.insert("beta".to_string(), Value::Number(value));
                preference_spec.insert("hyperParameters".to_string(), Value::Object(pref_hyper));
                body.insert(
                    "preferenceOptimizationSpec".to_string(),
                    Value::Object(preference_spec),
                );
            }
        }
    }

    if !hyper.is_empty() {
        spec.insert("hyperParameters".to_string(), Value::Object(hyper));
    }

    if let Some(export_last_checkpoint_only) = config.export_last_checkpoint_only {
        spec.insert(
            "exportLastCheckpointOnly".to_string(),
            Value::Bool(export_last_checkpoint_only),
        );
    }

    if let Some(evaluation_config) = &config.evaluation_config {
        spec.insert(
            "evaluationConfig".to_string(),
            serde_json::to_value(evaluation_config)?,
        );
    }

    if !spec.is_empty() {
        body.insert(spec_key.to_string(), Value::Object(spec));
    }

    if let Some(labels) = &config.labels {
        body.insert("labels".to_string(), serde_json::to_value(labels)?);
    }

    Ok(Value::Object(body))
}

fn parse_tuning_job_from_mldev(value: Value) -> Result<TuningJob> {
    let obj = value.as_object().ok_or_else(|| Error::Parse {
        message: "TuningJob response must be object".into(),
    })?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());
    let state = obj
        .get("state")
        .and_then(|v| v.as_str())
        .and_then(map_mldev_state);
    let create_time = obj
        .get("createTime")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());
    let update_time = obj
        .get("updateTime")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());
    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());
    let base_model = obj
        .get("baseModel")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    let tuning_task = obj.get("tuningTask").and_then(|v| v.as_object());
    let start_time = tuning_task
        .and_then(|task| task.get("startTime"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());
    let end_time = tuning_task
        .and_then(|task| task.get("completeTime"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    let tuned_model = name.clone().map(|n| TunedModel {
        model: Some(n.clone()),
        endpoint: Some(n),
        checkpoints: None,
    });

    Ok(TuningJob {
        name,
        state,
        create_time,
        start_time,
        end_time,
        update_time,
        error: None,
        description,
        base_model,
        tuned_model,
        pre_tuned_model: None,
        supervised_tuning_spec: None,
        preference_optimization_spec: None,
        tuning_data_stats: None,
        encryption_spec: None,
        partner_model_tuning_spec: None,
        evaluation_config: None,
        custom_base_model: None,
        experiment: None,
        labels: None,
        output_uri: None,
        pipeline_job: None,
        service_account: None,
        tuned_model_display_name: None,
        veo_tuning_spec: None,
    })
}

fn parse_list_tuning_jobs_from_mldev(value: Value) -> Result<ListTuningJobsResponse> {
    let obj = value.as_object().ok_or_else(|| Error::Parse {
        message: "ListTuningJobs response must be object".into(),
    })?;
    let next_page_token = obj
        .get("nextPageToken")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());
    let tuning_jobs = match obj.get("tunedModels") {
        Some(Value::Array(items)) => {
            let mut jobs = Vec::with_capacity(items.len());
            for item in items {
                jobs.push(parse_tuning_job_from_mldev(item.clone())?);
            }
            Some(jobs)
        }
        _ => None,
    };
    Ok(ListTuningJobsResponse {
        tuning_jobs,
        next_page_token,
    })
}

fn map_mldev_state(state: &str) -> Option<JobState> {
    match state {
        "STATE_UNSPECIFIED" => Some(JobState::JobStateUnspecified),
        "CREATING" => Some(JobState::JobStateRunning),
        "ACTIVE" => Some(JobState::JobStateSucceeded),
        "FAILED" => Some(JobState::JobStateFailed),
        _ => None,
    }
}

fn normalize_tuning_job_name(inner: &ClientInner, name: &str) -> Result<String> {
    match inner.config.backend {
        Backend::GeminiApi => {
            if name.starts_with("tunedModels/") {
                Ok(name.to_string())
            } else {
                Ok(format!("tunedModels/{name}"))
            }
        }
        Backend::VertexAi => {
            let vertex =
                inner
                    .config
                    .vertex_config
                    .as_ref()
                    .ok_or_else(|| Error::InvalidConfig {
                        message: "Vertex config missing".into(),
                    })?;
            if name.starts_with("projects/") {
                Ok(name.to_string())
            } else if name.starts_with("tuningJobs/") {
                Ok(format!(
                    "projects/{}/locations/{}/{}",
                    vertex.project, vertex.location, name
                ))
            } else {
                Ok(format!(
                    "projects/{}/locations/{}/tuningJobs/{}",
                    vertex.project, vertex.location, name
                ))
            }
        }
    }
}

fn build_tuning_job_url(
    inner: &ClientInner,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!("{base}{version}/{name}"))
}

fn build_tuning_job_cancel_url(
    inner: &ClientInner,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!("{base}{version}/{name}:cancel"))
}

fn build_tuning_jobs_url(
    inner: &ClientInner,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/tunedModels"),
        Backend::VertexAi => {
            let vertex =
                inner
                    .config
                    .vertex_config
                    .as_ref()
                    .ok_or_else(|| Error::InvalidConfig {
                        message: "Vertex config missing".into(),
                    })?;
            format!(
                "{base}{version}/projects/{}/locations/{}/tuningJobs",
                vertex.project, vertex.location
            )
        }
    };
    Ok(url)
}

fn add_list_query_params(url: String, config: &ListTuningJobsConfig) -> Result<String> {
    let mut url = reqwest::Url::parse(&url).map_err(|err| Error::InvalidConfig {
        message: err.to_string(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(page_size) = config.page_size {
            pairs.append_pair("pageSize", &page_size.to_string());
        }
        if let Some(page_token) = &config.page_token {
            pairs.append_pair("pageToken", page_token);
        }
        if let Some(filter) = &config.filter {
            pairs.append_pair("filter", filter);
        }
    }
    Ok(url.to_string())
}

fn transform_model_name(backend: Backend, model: &str) -> String {
    match backend {
        Backend::GeminiApi => {
            if model.starts_with("models/") || model.starts_with("tunedModels/") {
                model.to_string()
            } else {
                format!("models/{model}")
            }
        }
        Backend::VertexAi => {
            if model.starts_with("projects/") || model.starts_with("publishers/") {
                model.to_string()
            } else {
                format!("publishers/google/models/{model}")
            }
        }
    }
}

fn apply_http_options(
    mut request: reqwest::RequestBuilder,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<reqwest::RequestBuilder> {
    if let Some(options) = http_options {
        if let Some(timeout) = options.timeout {
            request = request.timeout(Duration::from_millis(timeout));
        }
        if let Some(headers) = &options.headers {
            for (key, value) in headers {
                let name =
                    HeaderName::from_bytes(key.as_bytes()).map_err(|_| Error::InvalidConfig {
                        message: format!("Invalid header name: {key}"),
                    })?;
                let value = HeaderValue::from_str(value).map_err(|_| Error::InvalidConfig {
                    message: format!("Invalid header value for {key}"),
                })?;
                request = request.header(name, value);
            }
        }
    }
    Ok(request)
}

fn merge_extra_body(
    body: &mut Value,
    http_options: &rust_genai_types::http::HttpOptions,
) -> Result<()> {
    if let Some(extra) = &http_options.extra_body {
        match (body, extra) {
            (Value::Object(body_map), Value::Object(extra_map)) => {
                for (key, value) in extra_map {
                    body_map.insert(key.clone(), value.clone());
                }
            }
            (_, _) => {
                return Err(Error::InvalidConfig {
                    message: "HttpOptions.extra_body must be an object".into(),
                });
            }
        }
    }
    Ok(())
}
