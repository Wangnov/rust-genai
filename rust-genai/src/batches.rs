//! Batches API surface.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::batches::{
    BatchJob, BatchJobDestination, BatchJobSource, CreateBatchJobConfig, DeleteBatchJobConfig,
    GetBatchJobConfig, InlinedRequest, ListBatchJobsConfig, ListBatchJobsResponse,
};
use rust_genai_types::enums::JobState;
use serde_json::{json, Map, Value};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct Batches {
    pub(crate) inner: Arc<ClientInner>,
}

impl Batches {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建批处理任务。
    pub async fn create(
        &self,
        model: impl Into<String>,
        src: BatchJobSource,
        mut config: CreateBatchJobConfig,
    ) -> Result<BatchJob> {
        let http_options = config.http_options.take();
        let model = normalize_batch_model(&self.inner, &model.into())?;

        let body = match self.inner.config.backend {
            Backend::GeminiApi => build_gemini_batch_body(&self.inner, &model, &src, &config)?,
            Backend::VertexAi => build_vertex_batch_body(&self.inner, &model, &src, &config)?,
        };

        let url = build_batch_create_url(&self.inner, &model, http_options.as_ref())?;
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
        parse_batch_job_response(&self.inner, value)
    }

    /// 获取批处理任务。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<BatchJob> {
        self.get_with_config(name, GetBatchJobConfig::default())
            .await
    }

    /// 获取批处理任务（带配置）。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetBatchJobConfig,
    ) -> Result<BatchJob> {
        let http_options = config.http_options.take();
        let name = normalize_batch_job_name(&self.inner, name.as_ref())?;
        let url = build_batch_job_url(&self.inner, &name, http_options.as_ref())?;
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
        parse_batch_job_response(&self.inner, value)
    }

    /// 删除批处理任务。
    pub async fn delete(&self, name: impl AsRef<str>) -> Result<()> {
        self.delete_with_config(name, DeleteBatchJobConfig::default())
            .await
    }

    /// 删除批处理任务（带配置）。
    pub async fn delete_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: DeleteBatchJobConfig,
    ) -> Result<()> {
        let http_options = config.http_options.take();
        let name = normalize_batch_job_name(&self.inner, name.as_ref())?;
        let url = build_batch_job_url(&self.inner, &name, http_options.as_ref())?;
        let mut request = self.inner.http.delete(url);
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

    /// 列出批处理任务。
    pub async fn list(&self) -> Result<ListBatchJobsResponse> {
        self.list_with_config(ListBatchJobsConfig::default()).await
    }

    /// 列出批处理任务（带配置）。
    pub async fn list_with_config(
        &self,
        mut config: ListBatchJobsConfig,
    ) -> Result<ListBatchJobsResponse> {
        let http_options = config.http_options.take();
        let url = build_batch_list_url(&self.inner, http_options.as_ref())?;
        let url = add_list_query_params(&self.inner, url, &config)?;
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
        parse_batch_job_list_response(&self.inner, value)
    }

    /// 列出所有批处理任务（自动翻页）。
    pub async fn all(&self) -> Result<Vec<BatchJob>> {
        self.all_with_config(ListBatchJobsConfig::default()).await
    }

    /// 列出所有批处理任务（带配置，自动翻页）。
    pub async fn all_with_config(&self, mut config: ListBatchJobsConfig) -> Result<Vec<BatchJob>> {
        let mut jobs = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options = http_options.clone();
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

fn normalize_batch_model(inner: &ClientInner, model: &str) -> Result<String> {
    match inner.config.backend {
        Backend::GeminiApi => {
            if model.starts_with("models/") || model.starts_with("tunedModels/") {
                Ok(model.to_string())
            } else {
                Ok(format!("models/{model}"))
            }
        }
        Backend::VertexAi => {
            if model.starts_with("projects/")
                || model.starts_with("publishers/")
                || model.starts_with("models/")
            {
                Ok(model.to_string())
            } else if let Some((publisher, name)) = model.split_once('/') {
                Ok(format!("publishers/{publisher}/models/{name}"))
            } else {
                Ok(format!("publishers/google/models/{model}"))
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
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!("{base}{version}/{name}"))
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
    url: String,
    config: &ListBatchJobsConfig,
) -> Result<String> {
    if inner.config.backend == Backend::GeminiApi && config.filter.is_some() {
        return Err(Error::InvalidConfig {
            message: "filter is not supported for Gemini API batch list".into(),
        });
    }
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
            Value::String(normalize_batch_model(inner, model)?),
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

fn parse_batch_job_response(inner: &ClientInner, value: Value) -> Result<BatchJob> {
    match inner.config.backend {
        Backend::GeminiApi => parse_batch_job_from_mldev(value),
        Backend::VertexAi => parse_batch_job_from_vertex(value),
    }
}

fn parse_batch_job_from_mldev(value: Value) -> Result<BatchJob> {
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let metadata = value.get("metadata").and_then(|v| v.as_object());
    let mut batch = BatchJob {
        name,
        ..Default::default()
    };

    if let Some(metadata) = metadata {
        batch.display_name = metadata
            .get("displayName")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        batch.state = metadata
            .get("state")
            .and_then(|v| serde_json::from_value::<JobState>(v.clone()).ok());
        batch.create_time = metadata
            .get("createTime")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        batch.end_time = metadata
            .get("endTime")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        batch.update_time = metadata
            .get("updateTime")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        batch.model = metadata
            .get("model")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        if let Some(output) = metadata.get("output") {
            batch.dest = parse_batch_destination_from_mldev(output);
        }
    }

    Ok(batch)
}

fn parse_batch_destination_from_mldev(value: &Value) -> Option<BatchJobDestination> {
    let output = value.as_object()?;
    let mut dest = BatchJobDestination {
        file_name: output
            .get("responsesFile")
            .and_then(|v| v.as_str())
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

fn parse_batch_job_from_vertex(value: Value) -> Result<BatchJob> {
    let mut batch = BatchJob::default();
    let obj = value.as_object().ok_or_else(|| Error::Parse {
        message: "BatchJob response must be object".into(),
    })?;

    batch.name = obj.get("name").and_then(|v| v.as_str()).map(str::to_string);
    batch.display_name = obj
        .get("displayName")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    batch.state = obj
        .get("state")
        .and_then(|v| serde_json::from_value::<JobState>(v.clone()).ok());
    batch.error = obj
        .get("error")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    batch.create_time = obj
        .get("createTime")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    batch.start_time = obj
        .get("startTime")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    batch.end_time = obj
        .get("endTime")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    batch.update_time = obj
        .get("updateTime")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    batch.model = obj
        .get("model")
        .and_then(|v| v.as_str())
        .map(str::to_string);

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
            .and_then(|v| v.as_str())
            .map(str::to_string),
        gcs_uri: obj
            .get("gcsSource")
            .and_then(|v| v.get("uris"))
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        bigquery_uri: obj
            .get("bigquerySource")
            .and_then(|v| v.get("inputUri"))
            .and_then(|v| v.as_str())
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
            .and_then(|v| v.as_str())
            .map(str::to_string),
        gcs_uri: obj
            .get("gcsDestination")
            .and_then(|v| v.get("outputUriPrefix"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        bigquery_uri: obj
            .get("bigqueryDestination")
            .and_then(|v| v.get("outputUri"))
            .and_then(|v| v.as_str())
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
    value: Value,
) -> Result<ListBatchJobsResponse> {
    let mut resp = ListBatchJobsResponse::default();
    let obj = value.as_object().ok_or_else(|| Error::Parse {
        message: "Batch list response must be object".into(),
    })?;
    resp.next_page_token = obj
        .get("nextPageToken")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    match inner.config.backend {
        Backend::GeminiApi => {
            if let Some(operations) = obj.get("operations").and_then(|v| v.as_array()) {
                let mut jobs = Vec::new();
                for op in operations {
                    if let Ok(job) = parse_batch_job_from_mldev(op.clone()) {
                        jobs.push(job);
                    }
                }
                resp.batch_jobs = Some(jobs);
            }
        }
        Backend::VertexAi => {
            if let Some(jobs) = obj.get("batchPredictionJobs").and_then(|v| v.as_array()) {
                let mut parsed = Vec::new();
                for job in jobs {
                    if let Ok(job) = parse_batch_job_from_vertex(job.clone()) {
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
    use crate::client::{ApiClient, Backend, ClientConfig, ClientInner, Credentials, HttpOptions};

    #[test]
    fn test_normalize_batch_job_name_gemini() {
        let config = ClientConfig {
            api_key: Some("test-key".into()),
            backend: Backend::GeminiApi,
            vertex_config: None,
            http_options: HttpOptions::default(),
            credentials: Credentials::ApiKey("test-key".into()),
            auth_scopes: Vec::new(),
        };
        let api_client = ApiClient::new(&config).unwrap();
        let inner = ClientInner {
            http: reqwest::Client::new(),
            config,
            api_client,
            auth_provider: None,
        };
        let name = normalize_batch_job_name(&inner, "abc-123").unwrap();
        assert_eq!(name, "batches/abc-123");
    }
}
