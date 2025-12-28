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
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建调优任务（默认配置）。
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
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
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
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
                build_tune_body_vertex(&self.inner, &base_model, &training_dataset, &config)?
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
            Backend::GeminiApi => parse_tuning_job_from_mldev(&value),
            Backend::VertexAi => Ok(serde_json::from_value(value)?),
        }
    }

    /// 获取调优任务。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<TuningJob> {
        self.get_with_config(name, GetTuningJobConfig::default())
            .await
    }

    /// 获取调优任务（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetTuningJobConfig,
    ) -> Result<TuningJob> {
        let http_options = config.http_options.take();
        let name = normalize_tuning_job_name(&self.inner, name.as_ref())?;
        let url = build_tuning_job_url(&self.inner, &name, http_options.as_ref());
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
            Backend::GeminiApi => parse_tuning_job_from_mldev(&value),
            Backend::VertexAi => Ok(serde_json::from_value(value)?),
        }
    }

    /// 列出调优任务。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list(&self) -> Result<ListTuningJobsResponse> {
        self.list_with_config(ListTuningJobsConfig::default()).await
    }

    /// 列出调优任务（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list_with_config(
        &self,
        mut config: ListTuningJobsConfig,
    ) -> Result<ListTuningJobsResponse> {
        let http_options = config.http_options.take();
        let url = build_tuning_jobs_url(&self.inner, http_options.as_ref())?;
        let url = add_list_query_params(&url, &config)?;
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
            Backend::GeminiApi => parse_list_tuning_jobs_from_mldev(&value),
            Backend::VertexAi => Ok(serde_json::from_value(value)?),
        }
    }

    /// 列出所有调优任务（自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all(&self) -> Result<Vec<TuningJob>> {
        self.all_with_config(ListTuningJobsConfig::default()).await
    }

    /// 列出所有调优任务（带配置，自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all_with_config(
        &self,
        mut config: ListTuningJobsConfig,
    ) -> Result<Vec<TuningJob>> {
        let mut jobs = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options.clone_from(&http_options);
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
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn cancel(&self, name: impl AsRef<str>) -> Result<()> {
        self.cancel_with_config(name, CancelTuningJobConfig::default())
            .await
    }

    /// 取消调优任务（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn cancel_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: CancelTuningJobConfig,
    ) -> Result<()> {
        let http_options = config.http_options.take();
        let name = normalize_tuning_job_name(&self.inner, name.as_ref())?;
        let url = build_tuning_job_cancel_url(&self.inner, &name, http_options.as_ref());
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
        if let Some(value) = serde_json::Number::from_f64(f64::from(learning_rate)) {
            hyper.insert("learningRate".to_string(), Value::Number(value));
        }
    }
    if let Some(learning_rate_multiplier) = config.learning_rate_multiplier {
        if let Some(value) = serde_json::Number::from_f64(f64::from(learning_rate_multiplier)) {
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
    training_dataset: &TuningDataset,
    config: &CreateTuningJobConfig,
) -> Result<Value> {
    validate_vertex_tuning_inputs(config, training_dataset)?;

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

    insert_vertex_dataset_uris(training_dataset, config, &mut spec);

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

    insert_vertex_hyper_params(config, method, &mut body, &mut spec)?;

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

fn validate_vertex_tuning_inputs(
    config: &CreateTuningJobConfig,
    training_dataset: &TuningDataset,
) -> Result<()> {
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
    Ok(())
}

fn insert_vertex_dataset_uris(
    training_dataset: &TuningDataset,
    config: &CreateTuningJobConfig,
    spec: &mut Map<String, Value>,
) {
    if let Some(uri) = training_dataset
        .gcs_uri
        .clone()
        .or_else(|| training_dataset.vertex_dataset_resource.clone())
    {
        spec.insert("trainingDatasetUri".to_string(), Value::String(uri));
    }

    if let Some(validation_dataset) = &config.validation_dataset {
        let validation_uri = validation_dataset
            .gcs_uri
            .clone()
            .or_else(|| validation_dataset.vertex_dataset_resource.clone());
        if let Some(uri) = validation_uri {
            spec.insert("validationDatasetUri".to_string(), Value::String(uri));
        }
    }
}

fn insert_vertex_hyper_params(
    config: &CreateTuningJobConfig,
    method: TuningMethod,
    body: &mut Map<String, Value>,
    spec: &mut Map<String, Value>,
) -> Result<()> {
    let mut hyper = Map::new();
    if let Some(epoch_count) = config.epoch_count {
        hyper.insert("epochCount".to_string(), Value::Number(epoch_count.into()));
    }
    if let Some(learning_rate_multiplier) = config.learning_rate_multiplier {
        if let Some(value) = serde_json::Number::from_f64(f64::from(learning_rate_multiplier)) {
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
        if let Some(value) = serde_json::Number::from_f64(f64::from(beta)) {
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
    Ok(())
}

fn parse_tuning_job_from_mldev(value: &Value) -> Result<TuningJob> {
    let obj = value.as_object().ok_or_else(|| Error::Parse {
        message: "TuningJob response must be object".into(),
    })?;

    let name = obj
        .get("name")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let state = obj
        .get("state")
        .and_then(Value::as_str)
        .and_then(map_mldev_state);
    let create_time = obj
        .get("createTime")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let update_time = obj
        .get("updateTime")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let description = obj
        .get("description")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let base_model = obj
        .get("baseModel")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let tuning_task = obj.get("tuningTask").and_then(Value::as_object);
    let start_time = tuning_task
        .and_then(|task| task.get("startTime"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let end_time = tuning_task
        .and_then(|task| task.get("completeTime"))
        .and_then(Value::as_str)
        .map(ToString::to_string);

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

fn parse_list_tuning_jobs_from_mldev(value: &Value) -> Result<ListTuningJobsResponse> {
    let obj = value.as_object().ok_or_else(|| Error::Parse {
        message: "ListTuningJobs response must be object".into(),
    })?;
    let next_page_token = obj
        .get("nextPageToken")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let tuning_jobs = match obj.get("tunedModels") {
        Some(Value::Array(items)) => {
            let mut jobs = Vec::with_capacity(items.len());
            for item in items {
                jobs.push(parse_tuning_job_from_mldev(item)?);
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
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/{name}")
}

fn build_tuning_job_cancel_url(
    inner: &ClientInner,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/{name}:cancel")
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

fn add_list_query_params(url: &str, config: &ListTuningJobsConfig) -> Result<String> {
    let mut url = reqwest::Url::parse(url).map_err(|err| Error::InvalidConfig {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        test_client_inner, test_client_inner_with_base, test_vertex_inner_missing_config,
    };
    use rust_genai_types::http::HttpOptions as TypesHttpOptions;
    use rust_genai_types::tunings::{
        EvaluationConfig, TuningDataset, TuningExample, TuningValidationDataset,
    };
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param, query_param_is_missing};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_validate_and_build_tune_body_mldev() {
        let config = CreateTuningJobConfig {
            validation_dataset: Some(TuningValidationDataset {
                gcs_uri: Some("gs://val".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = validate_mldev_config(&config).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let inner = test_client_inner(Backend::GeminiApi);
        let dataset = TuningDataset {
            examples: Some(vec![TuningExample {
                text_input: Some("hi".to_string()),
                output: Some("ok".to_string()),
            }]),
            ..Default::default()
        };
        let config = CreateTuningJobConfig {
            tuned_model_display_name: Some("demo".to_string()),
            epoch_count: Some(3),
            batch_size: Some(2),
            learning_rate: Some(0.01),
            learning_rate_multiplier: Some(0.5),
            ..Default::default()
        };
        let body = build_tune_body_mldev(&inner, "gemini-1.5-pro", dataset, &config).unwrap();
        assert_eq!(
            body.get("baseModel").and_then(Value::as_str),
            Some("models/gemini-1.5-pro")
        );
        assert!(body.get("examples").is_some());
        assert!(body.get("tuningTask").is_some());
    }

    #[test]
    fn test_validate_mldev_config_rejects_unsupported_fields() {
        let config = CreateTuningJobConfig {
            description: Some("desc".to_string()),
            ..Default::default()
        };
        assert!(validate_mldev_config(&config).is_err());

        let config = CreateTuningJobConfig {
            export_last_checkpoint_only: Some(true),
            ..Default::default()
        };
        assert!(validate_mldev_config(&config).is_err());

        let config = CreateTuningJobConfig {
            pre_tuned_model_checkpoint_id: Some("ckpt".to_string()),
            ..Default::default()
        };
        assert!(validate_mldev_config(&config).is_err());

        let config = CreateTuningJobConfig {
            adapter_size: Some(rust_genai_types::enums::AdapterSize::AdapterSizeFour),
            ..Default::default()
        };
        assert!(validate_mldev_config(&config).is_err());

        let config = CreateTuningJobConfig {
            evaluation_config: Some(rust_genai_types::tunings::EvaluationConfig {
                metrics: Some(vec![json!({"name": "metric"})]),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(validate_mldev_config(&config).is_err());

        let config = CreateTuningJobConfig {
            labels: Some([("k".to_string(), "v".to_string())].into()),
            ..Default::default()
        };
        assert!(validate_mldev_config(&config).is_err());

        let config = CreateTuningJobConfig {
            beta: Some(0.2),
            ..Default::default()
        };
        assert!(validate_mldev_config(&config).is_err());

        let inner = test_client_inner(Backend::GeminiApi);
        let dataset = TuningDataset {
            gcs_uri: Some("gs://train".to_string()),
            ..Default::default()
        };
        let err = build_tune_body_mldev(
            &inner,
            "gemini-1.5-pro",
            dataset,
            &CreateTuningJobConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_tune_body_vertex_variants() {
        let inner = test_client_inner(Backend::VertexAi);
        let config = CreateTuningJobConfig {
            batch_size: Some(8),
            ..Default::default()
        };
        let err =
            build_tune_body_vertex(&inner, "gemini-1.5-pro", &TuningDataset::default(), &config)
                .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let dataset = TuningDataset {
            gcs_uri: Some("gs://train".to_string()),
            ..Default::default()
        };
        let config = CreateTuningJobConfig {
            method: Some(TuningMethod::PreferenceTuning),
            tuned_model_display_name: Some("demo".to_string()),
            description: Some("desc".to_string()),
            epoch_count: Some(2),
            learning_rate_multiplier: Some(0.2),
            adapter_size: Some(rust_genai_types::enums::AdapterSize::AdapterSizeFour),
            beta: Some(0.3),
            ..Default::default()
        };
        let body =
            build_tune_body_vertex(&inner, "projects/p/models/m", &dataset, &config).unwrap();
        assert!(body.get("preTunedModel").is_some());
        assert!(body.get("preferenceOptimizationSpec").is_some());
    }

    #[test]
    fn test_parse_and_normalize_tuning_jobs() {
        let job = parse_tuning_job_from_mldev(&json!({
            "name": "tunedModels/1",
            "state": "ACTIVE",
            "createTime": "t1",
            "updateTime": "t2",
            "baseModel": "models/base",
            "tuningTask": {"startTime": "s1", "completeTime": "s2"}
        }))
        .unwrap();
        assert_eq!(job.state, Some(JobState::JobStateSucceeded));
        assert!(job.tuned_model.is_some());

        let list = parse_list_tuning_jobs_from_mldev(&json!({
            "tunedModels": [ { "name": "tunedModels/1" } ],
            "nextPageToken": "t"
        }))
        .unwrap();
        assert_eq!(list.tuning_jobs.as_ref().unwrap().len(), 1);

        assert_eq!(map_mldev_state("CREATING"), Some(JobState::JobStateRunning));
        assert_eq!(map_mldev_state("UNKNOWN"), None);

        let gemini = test_client_inner(Backend::GeminiApi);
        assert_eq!(
            normalize_tuning_job_name(&gemini, "abc").unwrap(),
            "tunedModels/abc"
        );
        let vertex = test_client_inner(Backend::VertexAi);
        assert_eq!(
            normalize_tuning_job_name(&vertex, "tuningJobs/1").unwrap(),
            "projects/proj/locations/loc/tuningJobs/1"
        );
    }

    #[test]
    fn test_merge_extra_body_error() {
        let mut body = json!({"a": 1});
        let mut options = rust_genai_types::http::HttpOptions {
            extra_body: Some(json!({"b": 2})),
            ..Default::default()
        };
        merge_extra_body(&mut body, &options).unwrap();
        assert_eq!(body.get("b").and_then(serde_json::Value::as_i64), Some(2));

        let mut bad = json!(["x"]);
        options.extra_body = Some(json!("bad"));
        let err = merge_extra_body(&mut bad, &options).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_tuning_urls_and_transform_model() {
        let gemini = test_client_inner(Backend::GeminiApi);
        let url = build_tuning_jobs_url(&gemini, None).unwrap();
        assert!(url.ends_with("/v1beta/tunedModels"));
        let url = build_tuning_job_url(&gemini, "tunedModels/1", None);
        assert!(url.ends_with("/v1beta/tunedModels/1"));
        let url = build_tuning_job_cancel_url(&gemini, "tunedModels/1", None);
        assert!(url.ends_with("/v1beta/tunedModels/1:cancel"));
        assert_eq!(
            transform_model_name(Backend::GeminiApi, "tunedModels/1"),
            "tunedModels/1"
        );

        let vertex = test_client_inner(Backend::VertexAi);
        let url = build_tuning_jobs_url(&vertex, None).unwrap();
        assert!(url.contains("/projects/proj/locations/loc/tuningJobs"));
        assert_eq!(
            transform_model_name(Backend::VertexAi, "gemini-1.5-pro"),
            "publishers/google/models/gemini-1.5-pro"
        );
    }

    #[test]
    fn test_add_list_query_params_and_apply_http_options() {
        let url = add_list_query_params(
            "https://example.com/tunedModels",
            &ListTuningJobsConfig {
                page_size: Some(2),
                page_token: Some("t".to_string()),
                filter: Some("state=ACTIVE".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(url.contains("pageSize=2"));
        assert!(url.contains("pageToken=t"));
        assert!(url.contains("filter=state%3DACTIVE"));

        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = rust_genai_types::http::HttpOptions {
            headers: Some([("bad header".to_string(), "v".to_string())].into()),
            ..Default::default()
        };
        let err = apply_http_options(request, Some(&options)).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_tune_body_vertex_additional_fields() {
        let inner = test_client_inner(Backend::VertexAi);
        let training_dataset = TuningDataset {
            gcs_uri: Some("gs://train".to_string()),
            ..Default::default()
        };
        let validation_dataset = TuningValidationDataset {
            gcs_uri: Some("gs://val".to_string()),
            ..Default::default()
        };
        let config = CreateTuningJobConfig {
            validation_dataset: Some(validation_dataset),
            beta: Some(0.2),
            export_last_checkpoint_only: Some(true),
            evaluation_config: Some(EvaluationConfig {
                metrics: Some(vec![json!({"name": "metric"})]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let body =
            build_tune_body_vertex(&inner, "gemini-1.5-pro", &training_dataset, &config).unwrap();
        let spec = body
            .get("supervisedTuningSpec")
            .and_then(Value::as_object)
            .unwrap();
        assert_eq!(
            spec.get("validationDatasetUri").and_then(Value::as_str),
            Some("gs://val")
        );
        assert_eq!(
            spec.get("exportLastCheckpointOnly")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert!(spec.get("evaluationConfig").is_some());
        assert!(body.get("preferenceOptimizationSpec").is_some());
    }

    #[test]
    fn test_parse_list_tuning_jobs_from_mldev_invalid() {
        let err = parse_list_tuning_jobs_from_mldev(&json!(["bad"])).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[test]
    fn test_vertex_missing_config_helpers() {
        let inner = test_vertex_inner_missing_config();
        let err = normalize_tuning_job_name(&inner, "tuningJobs/1").unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let err = build_tuning_jobs_url(&inner, None).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_add_list_query_params_invalid_url() {
        let err =
            add_list_query_params("http://[::1", &ListTuningJobsConfig::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_apply_http_options_invalid_header_value() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = TypesHttpOptions {
            headers: Some([("x-test".to_string(), "bad\nvalue".to_string())].into()),
            ..Default::default()
        };
        let err = apply_http_options(request, Some(&options)).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[tokio::test]
    async fn test_tunings_vertex_api_flow() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1beta1/projects/proj/locations/loc/tuningJobs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "projects/proj/locations/loc/tuningJobs/1",
                "state": "JOB_STATE_SUCCEEDED"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v1beta1/projects/proj/locations/loc/tuningJobs/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "projects/proj/locations/loc/tuningJobs/1",
                "state": "JOB_STATE_SUCCEEDED"
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/tuningJobs/1:cancel",
            ))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v1beta1/projects/proj/locations/loc/tuningJobs"))
            .and(query_param("pageSize", "2"))
            .and(query_param_is_missing("pageToken"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tuningJobs": [{
                    "name": "projects/proj/locations/loc/tuningJobs/1"
                }],
                "nextPageToken": "next"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v1beta1/projects/proj/locations/loc/tuningJobs"))
            .and(query_param("pageSize", "2"))
            .and(query_param("pageToken", "next"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tuningJobs": [{
                    "name": "projects/proj/locations/loc/tuningJobs/2"
                }]
            })))
            .mount(&server)
            .await;

        let inner = test_client_inner_with_base(Backend::VertexAi, &server.uri(), "v1beta1");
        let tunings = Tunings::new(Arc::new(inner));

        let dataset = TuningDataset {
            gcs_uri: Some("gs://train".to_string()),
            ..Default::default()
        };
        let created = tunings
            .tune_with_config("gemini-1.5-pro", dataset, CreateTuningJobConfig::default())
            .await
            .unwrap();
        assert!(created.name.as_deref().unwrap().contains("tuningJobs/1"));

        let got = tunings.get("tuningJobs/1").await.unwrap();
        assert!(got.name.as_deref().unwrap().contains("tuningJobs/1"));

        tunings.cancel("tuningJobs/1").await.unwrap();

        let list = tunings
            .list_with_config(ListTuningJobsConfig {
                page_size: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(list.tuning_jobs.unwrap().len(), 1);

        let all = tunings
            .all_with_config(ListTuningJobsConfig {
                page_size: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_build_tune_body_mldev_dataset_errors() {
        let inner = test_client_inner(Backend::GeminiApi);
        let dataset = TuningDataset {
            vertex_dataset_resource: Some("projects/p/locations/l/datasets/1".to_string()),
            ..Default::default()
        };
        let err = build_tune_body_mldev(
            &inner,
            "gemini-1.5-pro",
            dataset,
            &CreateTuningJobConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_tune_body_vertex_rejects_fields() {
        let inner = test_client_inner(Backend::VertexAi);
        let dataset = TuningDataset {
            gcs_uri: Some("gs://train".to_string()),
            ..Default::default()
        };
        let config = CreateTuningJobConfig {
            learning_rate: Some(0.1),
            ..Default::default()
        };
        let err = build_tune_body_vertex(&inner, "gemini-1.5-pro", &dataset, &config).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let dataset_with_examples = TuningDataset {
            examples: Some(vec![TuningExample {
                text_input: Some("hi".to_string()),
                output: Some("ok".to_string()),
            }]),
            ..Default::default()
        };
        let err = build_tune_body_vertex(
            &inner,
            "gemini-1.5-pro",
            &dataset_with_examples,
            &CreateTuningJobConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_tune_body_vertex_labels() {
        let inner = test_client_inner(Backend::VertexAi);
        let dataset = TuningDataset {
            gcs_uri: Some("gs://train".to_string()),
            ..Default::default()
        };
        let config = CreateTuningJobConfig {
            labels: Some([("k".to_string(), "v".to_string())].into()),
            ..Default::default()
        };
        let body = build_tune_body_vertex(&inner, "gemini-1.5-pro", &dataset, &config).unwrap();
        assert!(body.get("labels").is_some());
    }

    #[test]
    fn test_parse_tuning_job_from_mldev_invalid() {
        let err = parse_tuning_job_from_mldev(&json!(["bad"])).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[test]
    fn test_normalize_tuning_job_name_vertex_paths() {
        let inner = test_client_inner(Backend::VertexAi);
        let full =
            normalize_tuning_job_name(&inner, "projects/proj/locations/loc/tuningJobs/1").unwrap();
        assert!(full.starts_with("projects/proj/locations/loc"));

        let short = normalize_tuning_job_name(&inner, "tuningJobs/2").unwrap();
        assert!(short.contains("tuningJobs/2"));
    }

    #[test]
    fn test_apply_http_options_success_path() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = TypesHttpOptions {
            timeout: Some(1),
            headers: Some([("x-ok".to_string(), "v".to_string())].into()),
            ..Default::default()
        };
        let request = apply_http_options(request, Some(&options)).unwrap();
        let built = request.build().unwrap();
        assert!(built.headers().contains_key("x-ok"));
    }
}
