//! Batches API surface.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::batches::{
    BatchJob, BatchJobDestination, BatchJobSource, CancelBatchJobConfig, CreateBatchJobConfig,
    DeleteBatchJobConfig, DeleteResourceJob, GetBatchJobConfig, InlinedRequest, ListBatchJobsConfig,
    ListBatchJobsResponse,
};
use rust_genai_types::enums::JobState;
use serde_json::{json, Map, Value};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};
use crate::http_response::sdk_http_response_from_headers;

#[derive(Clone)]
pub struct Batches {
    pub(crate) inner: Arc<ClientInner>,
}

impl Batches {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建批处理任务。
    ///
    /// # Errors
    /// 当请求失败、服务端返回错误或响应解析失败时返回错误。
    pub async fn create(
        &self,
        model: impl Into<String>,
        src: BatchJobSource,
        mut config: CreateBatchJobConfig,
    ) -> Result<BatchJob> {
        let http_options = config.http_options.take();
        let model = normalize_batch_model(&self.inner, &model.into());

        let body = match self.inner.config.backend {
            Backend::GeminiApi => build_gemini_batch_body(&self.inner, &model, &src, &config)?,
            Backend::VertexAi => build_vertex_batch_body(&self.inner, &model, &src, &config)?,
        };

        let url = build_batch_create_url(&self.inner, &model, http_options.as_ref())?;
        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let value = response.json::<Value>().await?;
        parse_batch_job_response(&self.inner, &value)
    }

    /// 获取批处理任务。
    ///
    /// # Errors
    /// 当请求失败、服务端返回错误或响应解析失败时返回错误。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<BatchJob> {
        self.get_with_config(name, GetBatchJobConfig::default())
            .await
    }

    /// 获取批处理任务（带配置）。
    ///
    /// # Errors
    /// 当请求失败、服务端返回错误或响应解析失败时返回错误。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetBatchJobConfig,
    ) -> Result<BatchJob> {
        let http_options = config.http_options.take();
        let name = normalize_batch_job_name(&self.inner, name.as_ref())?;
        let url = build_batch_job_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let value = response.json::<Value>().await?;
        parse_batch_job_response(&self.inner, &value)
    }

    /// 删除批处理任务。
    ///
    /// # Errors
    /// 当请求失败或服务端返回错误时返回错误。
    pub async fn delete(&self, name: impl AsRef<str>) -> Result<DeleteResourceJob> {
        self.delete_with_config(name, DeleteBatchJobConfig::default())
            .await
    }

    /// 删除批处理任务（带配置）。
    ///
    /// # Errors
    /// 当请求失败或服务端返回错误时返回错误。
    pub async fn delete_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: DeleteBatchJobConfig,
    ) -> Result<DeleteResourceJob> {
        let http_options = config.http_options.take();
        let name = normalize_batch_job_name(&self.inner, name.as_ref())?;
        let url = build_batch_job_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.delete(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let headers = response.headers().clone();
        let text = response.text().await.unwrap_or_default();
        let mut result = if text.trim().is_empty() {
            DeleteResourceJob::default()
        } else {
            serde_json::from_str::<DeleteResourceJob>(&text)?
        };
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 取消批处理任务。
    ///
    /// # Errors
    /// 当请求失败或服务端返回错误时返回错误。
    pub async fn cancel(&self, name: impl AsRef<str>) -> Result<()> {
        self.cancel_with_config(name, CancelBatchJobConfig::default())
            .await
    }

    /// 取消批处理任务（带配置）。
    ///
    /// # Errors
    /// 当请求失败或服务端返回错误时返回错误。
    pub async fn cancel_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: CancelBatchJobConfig,
    ) -> Result<()> {
        let http_options = config.http_options.take();
        let name = normalize_batch_job_name(&self.inner, name.as_ref())?;
        let url = build_batch_job_cancel_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.post(url).json(&json!({}));
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(())
    }

    /// 列出批处理任务。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list(&self) -> Result<ListBatchJobsResponse> {
        self.list_with_config(ListBatchJobsConfig::default()).await
    }

    /// 列出批处理任务（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list_with_config(
        &self,
        mut config: ListBatchJobsConfig,
    ) -> Result<ListBatchJobsResponse> {
        let http_options = config.http_options.take();
        let url = build_batch_list_url(&self.inner, http_options.as_ref())?;
        let url = add_list_query_params(&self.inner, &url, &config)?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let headers = response.headers().clone();
        let value = response.json::<Value>().await?;
        let mut result = parse_batch_job_list_response(&self.inner, &value)?;
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 列出所有批处理任务（自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all(&self) -> Result<Vec<BatchJob>> {
        self.all_with_config(ListBatchJobsConfig::default()).await
    }

    /// 列出所有批处理任务（带配置，自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all_with_config(&self, mut config: ListBatchJobsConfig) -> Result<Vec<BatchJob>> {
        let mut jobs = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options.clone_from(&http_options);
            let response = self.list_with_config(page_config).await?;
            if let Some(items) = response.batch_jobs {
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
}

fn normalize_batch_model(inner: &ClientInner, model: &str) -> String {
    match inner.config.backend {
        Backend::GeminiApi => {
            if model.starts_with("models/") || model.starts_with("tunedModels/") {
                model.to_string()
            } else {
                format!("models/{model}")
            }
        }
        Backend::VertexAi => {
            if model.starts_with("projects/")
                || model.starts_with("publishers/")
                || model.starts_with("models/")
            {
                model.to_string()
            } else if let Some((publisher, name)) = model.split_once('/') {
                format!("publishers/{publisher}/models/{name}")
            } else {
                format!("publishers/google/models/{model}")
            }
        }
    }
}

fn normalize_batch_job_name(inner: &ClientInner, name: &str) -> Result<String> {
    match inner.config.backend {
        Backend::GeminiApi => {
            if name.starts_with("batches/") {
                Ok(name.to_string())
            } else {
                Ok(format!("batches/{name}"))
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
            } else if name.starts_with("locations/") {
                Ok(format!("projects/{}/{}", vertex.project, name))
            } else if name.starts_with("batchPredictionJobs/") {
                Ok(format!(
                    "projects/{}/locations/{}/{}",
                    vertex.project, vertex.location, name
                ))
            } else {
                Ok(format!(
                    "projects/{}/locations/{}/batchPredictionJobs/{}",
                    vertex.project, vertex.location, name
                ))
            }
        }
    }
}

fn build_batch_create_url(
    inner: &ClientInner,
    model: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/{model}:batchGenerateContent"),
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
                "{base}{version}/projects/{}/locations/{}/batchPredictionJobs",
                vertex.project, vertex.location
            )
        }
    };
    Ok(url)
}

fn build_batch_job_url(
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

fn build_batch_job_cancel_url(
    inner: &ClientInner,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    format!(
        "{}:cancel",
        build_batch_job_url(inner, name, http_options)
    )
}

fn build_batch_list_url(
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
        Backend::GeminiApi => format!("{base}{version}/batches"),
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
                "{base}{version}/projects/{}/locations/{}/batchPredictionJobs",
                vertex.project, vertex.location
            )
        }
    };
    Ok(url)
}

fn add_list_query_params(
    inner: &ClientInner,
    url: &str,
    config: &ListBatchJobsConfig,
) -> Result<String> {
    if inner.config.backend == Backend::GeminiApi && config.filter.is_some() {
        return Err(Error::InvalidConfig {
            message: "filter is not supported for Gemini API batch list".into(),
        });
    }
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

fn build_gemini_batch_body(
    inner: &ClientInner,
    _model: &str,
    src: &BatchJobSource,
    config: &CreateBatchJobConfig,
) -> Result<Value> {
    if config.dest.is_some() {
        return Err(Error::InvalidConfig {
            message: "dest is not supported in Gemini batch API".into(),
        });
    }
    let input_config = build_gemini_input_config(inner, src)?;
    let mut batch = Map::new();
    batch.insert("inputConfig".to_string(), input_config);
    if let Some(display_name) = &config.display_name {
        batch.insert(
            "displayName".to_string(),
            Value::String(display_name.clone()),
        );
    }
    Ok(Value::Object({
        let mut root = Map::new();
        root.insert("batch".to_string(), Value::Object(batch));
        root
    }))
}

fn build_vertex_batch_body(
    _inner: &ClientInner,
    model: &str,
    src: &BatchJobSource,
    config: &CreateBatchJobConfig,
) -> Result<Value> {
    let input_config = build_vertex_input_config(src)?;
    let dest = config.dest.as_ref().ok_or_else(|| Error::InvalidConfig {
        message: "dest is required for Vertex batch API".into(),
    })?;
    let output_config = build_vertex_output_config(dest)?;

    let mut body = Map::new();
    body.insert("model".to_string(), Value::String(model.to_string()));
    body.insert("inputConfig".to_string(), input_config);
    body.insert("outputConfig".to_string(), output_config);
    if let Some(display_name) = &config.display_name {
        body.insert(
            "displayName".to_string(),
            Value::String(display_name.clone()),
        );
    }
    Ok(Value::Object(body))
}

fn build_gemini_input_config(inner: &ClientInner, src: &BatchJobSource) -> Result<Value> {
    if src.format.is_some() || src.gcs_uri.is_some() || src.bigquery_uri.is_some() {
        return Err(Error::InvalidConfig {
            message: "format/gcs_uri/bigquery_uri are not supported in Gemini batch API".into(),
        });
    }
    let mut config = Map::new();
    if let Some(file_name) = &src.file_name {
        config.insert("fileName".to_string(), Value::String(file_name.clone()));
    }
    if let Some(inlined) = &src.inlined_requests {
        let requests = build_inlined_requests(inner, inlined)?;
        config.insert("requests".to_string(), json!({ "requests": requests }));
    }
    if config.is_empty() {
        return Err(Error::InvalidConfig {
            message: "BatchJobSource requires file_name or inlined_requests".into(),
        });
    }
    Ok(Value::Object(config))
}

fn build_vertex_input_config(src: &BatchJobSource) -> Result<Value> {
    if src.file_name.is_some() || src.inlined_requests.is_some() {
        return Err(Error::InvalidConfig {
            message: "file_name/inlined_requests are not supported in Vertex batch API".into(),
        });
    }
    let mut config = Map::new();
    if let Some(format) = &src.format {
        config.insert("instancesFormat".to_string(), Value::String(format.clone()));
    }
    if let Some(gcs_uri) = &src.gcs_uri {
        config.insert("gcsSource".to_string(), json!({ "uris": gcs_uri }));
    }
    if let Some(bigquery_uri) = &src.bigquery_uri {
        config.insert(
            "bigquerySource".to_string(),
            json!({ "inputUri": bigquery_uri }),
        );
    }
    if config.is_empty() {
        return Err(Error::InvalidConfig {
            message: "BatchJobSource requires format + gcs_uri/bigquery_uri for Vertex".into(),
        });
    }
    Ok(Value::Object(config))
}

fn build_vertex_output_config(dest: &BatchJobDestination) -> Result<Value> {
    if dest.file_name.is_some()
        || dest.inlined_responses.is_some()
        || dest.inlined_embed_content_responses.is_some()
    {
        return Err(Error::InvalidConfig {
            message: "file_name/inlined_responses are not supported in Vertex batch API".into(),
        });
    }
    let mut config = Map::new();
    if let Some(format) = &dest.format {
        config.insert(
            "predictionsFormat".to_string(),
            Value::String(format.clone()),
        );
    }
    if let Some(gcs_uri) = &dest.gcs_uri {
        config.insert(
            "gcsDestination".to_string(),
            json!({ "outputUriPrefix": gcs_uri }),
        );
    }
    if let Some(bigquery_uri) = &dest.bigquery_uri {
        config.insert(
            "bigqueryDestination".to_string(),
            json!({ "outputUri": bigquery_uri }),
        );
    }
    if config.is_empty() {
        return Err(Error::InvalidConfig {
            message: "BatchJobDestination requires format + gcs_uri/bigquery_uri for Vertex".into(),
        });
    }
    Ok(Value::Object(config))
}

fn build_inlined_requests(inner: &ClientInner, requests: &[InlinedRequest]) -> Result<Vec<Value>> {
    requests
        .iter()
        .map(|req| build_inlined_request(inner, req))
        .collect()
}

fn build_inlined_request(inner: &ClientInner, request: &InlinedRequest) -> Result<Value> {
    let mut entry = Map::new();
    let mut request_map = Map::new();

    if let Some(model) = &request.model {
        request_map.insert(
            "model".to_string(),
            Value::String(normalize_batch_model(inner, model)),
        );
    }
    if let Some(contents) = &request.contents {
        request_map.insert("contents".to_string(), serde_json::to_value(contents)?);
    }
    if let Some(config) = &request.config {
        if let Some(generation) = &config.generation_config {
            request_map.insert(
                "generationConfig".to_string(),
                serde_json::to_value(generation)?,
            );
        }
    }
    entry.insert("request".to_string(), Value::Object(request_map));

    if let Some(metadata) = &request.metadata {
        entry.insert("metadata".to_string(), serde_json::to_value(metadata)?);
    }
    Ok(Value::Object(entry))
}

fn parse_batch_job_response(inner: &ClientInner, value: &Value) -> Result<BatchJob> {
    match inner.config.backend {
        Backend::GeminiApi => Ok(parse_batch_job_from_mldev(value)),
        Backend::VertexAi => parse_batch_job_from_vertex(value),
    }
}

fn parse_batch_job_from_mldev(value: &Value) -> BatchJob {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let metadata = value.get("metadata").and_then(Value::as_object);
    let mut batch = BatchJob {
        name,
        ..Default::default()
    };

    if let Some(metadata) = metadata {
        batch.display_name = metadata
            .get("displayName")
            .and_then(Value::as_str)
            .map(str::to_string);
        batch.state = metadata
            .get("state")
            .and_then(|v| serde_json::from_value::<JobState>(v.clone()).ok());
        batch.create_time = metadata
            .get("createTime")
            .and_then(Value::as_str)
            .map(str::to_string);
        batch.end_time = metadata
            .get("endTime")
            .and_then(Value::as_str)
            .map(str::to_string);
        batch.update_time = metadata
            .get("updateTime")
            .and_then(Value::as_str)
            .map(str::to_string);
        batch.model = metadata
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(output) = metadata.get("output") {
            batch.dest = parse_batch_destination_from_mldev(output);
        }
    }

    batch
}

fn parse_batch_destination_from_mldev(value: &Value) -> Option<BatchJobDestination> {
    let output = value.as_object()?;
    let mut dest = BatchJobDestination {
        file_name: output
            .get("responsesFile")
            .and_then(Value::as_str)
            .map(str::to_string),
        ..Default::default()
    };

    if let Some(inlined) = output
        .get("inlinedResponses")
        .and_then(|v| v.get("inlinedResponses"))
    {
        dest.inlined_responses = serde_json::from_value(inlined.clone()).ok();
    }
    if let Some(inlined) = output
        .get("inlinedEmbedContentResponses")
        .and_then(|v| v.get("inlinedResponses"))
    {
        dest.inlined_embed_content_responses = serde_json::from_value(inlined.clone()).ok();
    }
    Some(dest)
}

fn parse_batch_job_from_vertex(value: &Value) -> Result<BatchJob> {
    let mut batch = BatchJob::default();
    let obj = value.as_object().ok_or_else(|| Error::Parse {
        message: "BatchJob response must be object".into(),
    })?;

    batch.name = obj.get("name").and_then(Value::as_str).map(str::to_string);
    batch.display_name = obj
        .get("displayName")
        .and_then(Value::as_str)
        .map(str::to_string);
    batch.state = obj
        .get("state")
        .and_then(|v| serde_json::from_value::<JobState>(v.clone()).ok());
    batch.error = obj
        .get("error")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    batch.create_time = obj
        .get("createTime")
        .and_then(Value::as_str)
        .map(str::to_string);
    batch.start_time = obj
        .get("startTime")
        .and_then(Value::as_str)
        .map(str::to_string);
    batch.end_time = obj
        .get("endTime")
        .and_then(Value::as_str)
        .map(str::to_string);
    batch.update_time = obj
        .get("updateTime")
        .and_then(Value::as_str)
        .map(str::to_string);
    batch.model = obj.get("model").and_then(Value::as_str).map(str::to_string);

    if let Some(input) = obj.get("inputConfig") {
        batch.src = parse_batch_source_from_vertex(input);
    }
    if let Some(output) = obj.get("outputConfig") {
        batch.dest = parse_batch_destination_from_vertex(output);
    }
    if let Some(stats) = obj.get("completionStats") {
        batch.completion_stats = serde_json::from_value(stats.clone()).ok();
    }

    Ok(batch)
}

fn parse_batch_source_from_vertex(value: &Value) -> Option<BatchJobSource> {
    let obj = value.as_object()?;
    let src = BatchJobSource {
        format: obj
            .get("instancesFormat")
            .and_then(Value::as_str)
            .map(str::to_string),
        gcs_uri: obj
            .get("gcsSource")
            .and_then(|v| v.get("uris"))
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        bigquery_uri: obj
            .get("bigquerySource")
            .and_then(|v| v.get("inputUri"))
            .and_then(Value::as_str)
            .map(str::to_string),
        ..Default::default()
    };
    if src.format.is_none() && src.gcs_uri.is_none() && src.bigquery_uri.is_none() {
        None
    } else {
        Some(src)
    }
}

fn parse_batch_destination_from_vertex(value: &Value) -> Option<BatchJobDestination> {
    let obj = value.as_object()?;
    let dest = BatchJobDestination {
        format: obj
            .get("predictionsFormat")
            .and_then(Value::as_str)
            .map(str::to_string),
        gcs_uri: obj
            .get("gcsDestination")
            .and_then(|v| v.get("outputUriPrefix"))
            .and_then(Value::as_str)
            .map(str::to_string),
        bigquery_uri: obj
            .get("bigqueryDestination")
            .and_then(|v| v.get("outputUri"))
            .and_then(Value::as_str)
            .map(str::to_string),
        ..Default::default()
    };
    if dest.format.is_none() && dest.gcs_uri.is_none() && dest.bigquery_uri.is_none() {
        None
    } else {
        Some(dest)
    }
}

fn parse_batch_job_list_response(
    inner: &ClientInner,
    value: &Value,
) -> Result<ListBatchJobsResponse> {
    let mut resp = ListBatchJobsResponse::default();
    let obj = value.as_object().ok_or_else(|| Error::Parse {
        message: "Batch list response must be object".into(),
    })?;
    resp.next_page_token = obj
        .get("nextPageToken")
        .and_then(Value::as_str)
        .map(str::to_string);

    match inner.config.backend {
        Backend::GeminiApi => {
            if let Some(operations) = obj.get("operations").and_then(Value::as_array) {
                let mut jobs = Vec::new();
                for op in operations {
                    jobs.push(parse_batch_job_from_mldev(op));
                }
                resp.batch_jobs = Some(jobs);
            }
        }
        Backend::VertexAi => {
            if let Some(jobs) = obj.get("batchPredictionJobs").and_then(Value::as_array) {
                let mut parsed = Vec::new();
                for job in jobs {
                    if let Ok(job) = parse_batch_job_from_vertex(job) {
                        parsed.push(job);
                    }
                }
                resp.batch_jobs = Some(parsed);
            }
        }
    }

    Ok(resp)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        test_client_inner, test_client_inner_with_base, test_vertex_inner_missing_config,
    };
    use rust_genai_types::config::GenerationConfig;
    use rust_genai_types::content::Content;
    use rust_genai_types::models::GenerateContentConfig;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param, query_param_is_missing};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_normalize_batch_job_name_gemini() {
        let inner = test_client_inner(Backend::GeminiApi);
        let name = normalize_batch_job_name(&inner, "abc-123").unwrap();
        assert_eq!(name, "batches/abc-123");
    }

    #[test]
    fn test_normalize_batch_model_and_job_vertex() {
        let inner = test_client_inner(Backend::VertexAi);
        let model = normalize_batch_model(&inner, "gemini-1.5-pro");
        assert_eq!(model, "publishers/google/models/gemini-1.5-pro");
        let model = normalize_batch_model(&inner, "acme/custom");
        assert_eq!(model, "publishers/acme/models/custom");

        assert_eq!(
            normalize_batch_job_name(&inner, "projects/x/locations/y/batchPredictionJobs/z")
                .unwrap(),
            "projects/x/locations/y/batchPredictionJobs/z"
        );
        assert_eq!(
            normalize_batch_job_name(&inner, "locations/us/batchPredictionJobs/1").unwrap(),
            "projects/proj/locations/us/batchPredictionJobs/1"
        );
        assert_eq!(
            normalize_batch_job_name(&inner, "batchPredictionJobs/2").unwrap(),
            "projects/proj/locations/loc/batchPredictionJobs/2"
        );
        assert_eq!(
            normalize_batch_job_name(&inner, "job-3").unwrap(),
            "projects/proj/locations/loc/batchPredictionJobs/job-3"
        );
    }

    #[test]
    fn test_build_batch_configs_and_inlined_requests() {
        let inner = test_client_inner(Backend::GeminiApi);
        let mut bad_src = BatchJobSource::default();
        let err = build_gemini_input_config(&inner, &bad_src).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
        bad_src.format = Some("jsonl".to_string());
        let err = build_gemini_input_config(&inner, &bad_src).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let src = BatchJobSource {
            inlined_requests: Some(vec![InlinedRequest {
                model: Some("gemini-1.5-pro".to_string()),
                contents: Some(vec![Content::text("hello")]),
                metadata: Some([("k".to_string(), "v".to_string())].into()),
                config: None,
            }]),
            ..Default::default()
        };
        let input = build_gemini_input_config(&inner, &src).unwrap();
        assert!(input.get("requests").is_some());

        let vertex_src = BatchJobSource {
            format: Some("jsonl".to_string()),
            gcs_uri: Some(vec!["gs://in".to_string()]),
            ..Default::default()
        };
        let input = build_vertex_input_config(&vertex_src).unwrap();
        assert!(input.get("gcsSource").is_some());

        let bad_vertex_src = BatchJobSource {
            file_name: Some("file.jsonl".to_string()),
            ..Default::default()
        };
        let err = build_vertex_input_config(&bad_vertex_src).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let dest = BatchJobDestination {
            format: Some("jsonl".to_string()),
            gcs_uri: Some("gs://out".to_string()),
            ..Default::default()
        };
        let output = build_vertex_output_config(&dest).unwrap();
        assert!(output.get("gcsDestination").is_some());

        let bad_dest = BatchJobDestination {
            file_name: Some("out.jsonl".to_string()),
            ..Default::default()
        };
        let err = build_vertex_output_config(&bad_dest).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_parse_batch_job_and_list() {
        let gemini = test_client_inner(Backend::GeminiApi);
        let job = parse_batch_job_from_mldev(&json!({
            "name": "batches/1",
            "metadata": {
                "displayName": "job",
                "state": "JOB_STATE_SUCCEEDED",
                "createTime": "t1",
                "endTime": "t2",
                "updateTime": "t3",
                "model": "models/1",
                "output": {
                    "responsesFile": "file.jsonl"
                }
            }
        }));
        assert_eq!(job.display_name.as_deref(), Some("job"));
        assert_eq!(job.dest.unwrap().file_name.as_deref(), Some("file.jsonl"));

        let vertex = test_client_inner(Backend::VertexAi);
        let vertex_job = parse_batch_job_from_vertex(&json!({
            "name": "projects/proj/locations/loc/batchPredictionJobs/1",
            "displayName": "job",
            "state": "JOB_STATE_RUNNING",
            "inputConfig": {"instancesFormat": "jsonl"},
            "outputConfig": {"predictionsFormat": "jsonl", "gcsDestination": {"outputUriPrefix": "gs://out"}},
            "completionStats": {"successfulCount": "1"}
        }))
        .unwrap();
        assert_eq!(vertex_job.src.unwrap().format.as_deref(), Some("jsonl"));
        assert_eq!(
            vertex_job.dest.unwrap().gcs_uri.as_deref(),
            Some("gs://out")
        );

        let list = parse_batch_job_list_response(
            &gemini,
            &json!({"operations": [ {"name": "batches/1"} ], "nextPageToken": "t"}),
        )
        .unwrap();
        assert_eq!(list.batch_jobs.as_ref().unwrap().len(), 1);

        let list = parse_batch_job_list_response(
            &vertex,
            &json!({"batchPredictionJobs": [ {"name": "projects/p/locations/l/batchPredictionJobs/1"} ]}),
        )
        .unwrap();
        assert_eq!(list.batch_jobs.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_add_list_query_params_errors() {
        let inner = test_client_inner(Backend::GeminiApi);
        let err = add_list_query_params(
            &inner,
            "https://example.com/batches",
            &ListBatchJobsConfig {
                filter: Some("state=RUNNING".to_string()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_batch_urls() {
        let gemini = test_client_inner(Backend::GeminiApi);
        let url = build_batch_create_url(&gemini, "models/1", None).unwrap();
        assert!(url.ends_with("/v1beta/models/1:batchGenerateContent"));
        let url = build_batch_list_url(&gemini, None).unwrap();
        assert!(url.ends_with("/v1beta/batches"));
        let url = build_batch_job_url(&gemini, "batches/1", None);
        assert!(url.ends_with("/v1beta/batches/1"));

        let vertex = test_client_inner(Backend::VertexAi);
        let url = build_batch_create_url(&vertex, "publishers/google/models/1", None).unwrap();
        assert!(url.contains("/projects/proj/locations/loc/batchPredictionJobs"));
        let url = build_batch_list_url(&vertex, None).unwrap();
        assert!(url.contains("/projects/proj/locations/loc/batchPredictionJobs"));
    }

    #[test]
    fn test_build_batch_urls_vertex_missing_config() {
        let inner = test_vertex_inner_missing_config();
        let err = build_batch_create_url(&inner, "publishers/google/models/1", None).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
        let err = build_batch_list_url(&inner, None).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_add_list_query_params_invalid_url_and_vertex_filter() {
        let inner = test_client_inner(Backend::GeminiApi);
        let err = add_list_query_params(&inner, "://bad-url", &ListBatchJobsConfig::default())
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let inner = test_client_inner(Backend::VertexAi);
        let url = add_list_query_params(
            &inner,
            "https://example.com/batchPredictionJobs",
            &ListBatchJobsConfig {
                filter: Some("state=JOB_STATE_RUNNING".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(url.contains("filter=state%3DJOB_STATE_RUNNING"));
    }

    #[test]
    fn test_build_batch_body_includes_display_name_and_config() {
        let inner = test_client_inner(Backend::GeminiApi);
        let request = InlinedRequest {
            model: Some("gemini-1.5-pro".to_string()),
            contents: Some(vec![Content::text("hello")]),
            metadata: Some([("key".to_string(), "value".to_string())].into()),
            config: Some(GenerateContentConfig {
                generation_config: Some(GenerationConfig {
                    temperature: Some(0.2),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        };
        let src = BatchJobSource {
            inlined_requests: Some(vec![request]),
            ..Default::default()
        };
        let body = build_gemini_batch_body(
            &inner,
            "models/1",
            &src,
            &CreateBatchJobConfig {
                display_name: Some("batch-name".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(body
            .get("batch")
            .and_then(|v| v.get("displayName"))
            .is_some());
        let reqs = body
            .get("batch")
            .and_then(|v| v.get("inputConfig"))
            .and_then(|v| v.get("requests"))
            .and_then(|v| v.get("requests"))
            .and_then(Value::as_array)
            .unwrap();
        let request = &reqs[0]["request"];
        assert!(request.get("generationConfig").is_some());
        assert!(reqs[0].get("metadata").is_some());
    }

    #[test]
    fn test_vertex_input_output_config_bigquery_and_empty_errors() {
        let src = BatchJobSource {
            format: Some("jsonl".to_string()),
            bigquery_uri: Some("bq://project.dataset.table".to_string()),
            ..Default::default()
        };
        let input = build_vertex_input_config(&src).unwrap();
        assert!(input.get("bigquerySource").is_some());

        let dest = BatchJobDestination {
            format: Some("jsonl".to_string()),
            bigquery_uri: Some("bq://project.dataset.output".to_string()),
            ..Default::default()
        };
        let output = build_vertex_output_config(&dest).unwrap();
        assert!(output.get("bigqueryDestination").is_some());

        let err = build_vertex_input_config(&BatchJobSource::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
        let err = build_vertex_output_config(&BatchJobDestination::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_parse_batch_job_from_vertex_errors_and_empty_sources() {
        let err = parse_batch_job_from_vertex(&Value::Null).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));

        let empty = json!({});
        assert!(parse_batch_source_from_vertex(&empty).is_none());
        assert!(parse_batch_destination_from_vertex(&empty).is_none());
    }

    #[test]
    fn test_parse_batch_list_response_invalid_value() {
        let inner = test_client_inner(Backend::GeminiApi);
        let err = parse_batch_job_list_response(&inner, &Value::Null).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[test]
    fn test_apply_http_options_success_path() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = rust_genai_types::http::HttpOptions {
            timeout: Some(500),
            headers: Some([("x-ok".to_string(), "ok".to_string())].into()),
            ..Default::default()
        };
        let request = apply_http_options(request, Some(&options)).unwrap();
        let built = request.build().unwrap();
        assert_eq!(built.headers().get("x-ok").unwrap(), "ok");
    }

    #[test]
    fn test_apply_http_options_invalid_header_value() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = rust_genai_types::http::HttpOptions {
            headers: Some([("x-test".to_string(), "bad\nvalue".to_string())].into()),
            ..Default::default()
        };
        let err = apply_http_options(request, Some(&options)).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_normalize_batch_job_name_vertex_missing_config() {
        let inner = test_vertex_inner_missing_config();
        let err = normalize_batch_job_name(&inner, "job").unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_batch_bodies_errors() {
        let inner = test_client_inner(Backend::GeminiApi);
        let src = BatchJobSource {
            file_name: Some("file.jsonl".to_string()),
            ..Default::default()
        };
        let err = build_gemini_batch_body(
            &inner,
            "models/1",
            &src,
            &CreateBatchJobConfig {
                dest: Some(BatchJobDestination::default()),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let inner = test_client_inner(Backend::VertexAi);
        let err = build_vertex_batch_body(
            &inner,
            "models/1",
            &BatchJobSource {
                format: Some("jsonl".to_string()),
                gcs_uri: Some(vec!["gs://input".to_string()]),
                ..Default::default()
            },
            &CreateBatchJobConfig::default(),
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_parse_batch_destination_mldev_inlined() {
        let dest = parse_batch_destination_from_mldev(&json!({
            "responsesFile": "file.jsonl",
            "inlinedResponses": {"inlinedResponses": [{"response": {"candidates": []}}]},
            "inlinedEmbedContentResponses": {"inlinedResponses": [{"response": {"embedding": null}}]}
        }))
        .unwrap();
        assert_eq!(dest.file_name.as_deref(), Some("file.jsonl"));
        assert!(dest.inlined_responses.is_some());
        assert!(dest.inlined_embed_content_responses.is_some());
    }

    #[tokio::test]
    async fn test_batches_vertex_api_flow_create_get_delete() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/batchPredictionJobs",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "projects/proj/locations/loc/batchPredictionJobs/1",
                "state": "JOB_STATE_SUCCEEDED",
                "inputConfig": {
                    "instancesFormat": "jsonl",
                    "gcsSource": {"uris": ["gs://input"]}
                },
                "outputConfig": {
                    "predictionsFormat": "jsonl",
                    "gcsDestination": {"outputUriPrefix": "gs://out"}
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/batchPredictionJobs/1",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "projects/proj/locations/loc/batchPredictionJobs/1",
                "state": "JOB_STATE_SUCCEEDED"
            })))
            .mount(&server)
            .await;

        Mock::given(method("DELETE"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/batchPredictionJobs/1",
            ))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/batchPredictionJobs/1:cancel",
            ))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let inner = test_client_inner_with_base(Backend::VertexAi, &server.uri(), "v1beta1");
        let batches = Batches::new(Arc::new(inner));

        let created = batches
            .create(
                "gemini-1.5-pro",
                BatchJobSource {
                    format: Some("jsonl".to_string()),
                    gcs_uri: Some(vec!["gs://input".to_string()]),
                    ..Default::default()
                },
                CreateBatchJobConfig {
                    dest: Some(BatchJobDestination {
                        format: Some("jsonl".to_string()),
                        gcs_uri: Some("gs://out".to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(created
            .name
            .as_deref()
            .unwrap()
            .contains("batchPredictionJobs/1"));

        let job = batches.get("1").await.unwrap();
        assert!(job
            .name
            .as_deref()
            .unwrap()
            .contains("batchPredictionJobs/1"));

        batches.cancel("1").await.unwrap();

        batches.delete("1").await.unwrap();
    }

    #[tokio::test]
    async fn test_batches_vertex_api_flow_list_and_all() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/batchPredictionJobs",
            ))
            .and(query_param("pageSize", "2"))
            .and(query_param_is_missing("pageToken"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "batchPredictionJobs": [{
                    "name": "projects/proj/locations/loc/batchPredictionJobs/1"
                }],
                "nextPageToken": "next"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/batchPredictionJobs",
            ))
            .and(query_param("pageSize", "2"))
            .and(query_param("pageToken", "next"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "batchPredictionJobs": [{
                    "name": "projects/proj/locations/loc/batchPredictionJobs/2"
                }]
            })))
            .mount(&server)
            .await;

        let inner = test_client_inner_with_base(Backend::VertexAi, &server.uri(), "v1beta1");
        let batches = Batches::new(Arc::new(inner));

        let list = batches
            .list_with_config(ListBatchJobsConfig {
                page_size: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(list.batch_jobs.unwrap().len(), 1);

        let all = batches
            .all_with_config(ListBatchJobsConfig {
                page_size: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
    }
}
