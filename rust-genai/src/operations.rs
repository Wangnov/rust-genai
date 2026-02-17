//! Operations API surface.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::file_search_stores::{ImportFileOperation, UploadToFileSearchStoreOperation};
use rust_genai_types::models::GenerateVideosOperation;
use rust_genai_types::operations::{
    GetOperationConfig, ListOperationsConfig, ListOperationsResponse, Operation,
};
use serde_json::Value;

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct Operations {
    pub(crate) inner: Arc<ClientInner>,
}

impl Operations {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 获取操作状态。
    ///
    /// # Errors
    /// 当请求失败、服务端返回错误或响应解析失败时返回错误。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<Operation> {
        self.get_with_config(name, GetOperationConfig::default())
            .await
    }

    /// 获取操作状态（带配置）。
    ///
    /// # Errors
    /// 当请求失败、服务端返回错误或响应解析失败时返回错误。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetOperationConfig,
    ) -> Result<Operation> {
        let http_options = config.http_options.take();
        let name = normalize_operation_name(&self.inner, name.as_ref())?;
        let url = build_operation_url(&self.inner, &name, http_options.as_ref());
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
        Ok(response.json::<Operation>().await?)
    }

    /// 列出操作。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list(&self) -> Result<ListOperationsResponse> {
        self.list_with_config(ListOperationsConfig::default()).await
    }

    /// 列出操作（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list_with_config(
        &self,
        mut config: ListOperationsConfig,
    ) -> Result<ListOperationsResponse> {
        let http_options = config.http_options.take();
        let url = build_operations_list_url(&self.inner, http_options.as_ref())?;
        let url = add_list_query_params(&url, &config)?;
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
        Ok(response.json::<ListOperationsResponse>().await?)
    }

    /// 列出所有操作（自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all(&self) -> Result<Vec<Operation>> {
        self.all_with_config(ListOperationsConfig::default()).await
    }

    /// 列出所有操作（带配置，自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all_with_config(
        &self,
        mut config: ListOperationsConfig,
    ) -> Result<Vec<Operation>> {
        let mut ops = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options.clone_from(&http_options);
            let response = self.list_with_config(page_config).await?;
            if let Some(items) = response.operations {
                ops.extend(items);
            }
            match response.next_page_token {
                Some(token) if !token.is_empty() => {
                    config.page_token = Some(token);
                }
                _ => break,
            }
        }
        Ok(ops)
    }

    /// 等待操作完成（轮询）。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或轮询过程中响应解析失败时返回错误。
    pub async fn wait(&self, mut operation: Operation) -> Result<Operation> {
        let name = operation.name.clone().ok_or_else(|| Error::InvalidConfig {
            message: "Operation name is empty".into(),
        })?;
        while !operation.done.unwrap_or(false) {
            tokio::time::sleep(Duration::from_secs(5)).await;
            operation = self.get(&name).await?;
        }
        Ok(operation)
    }

    /// 获取 `GenerateVideos` 操作状态。
    ///
    /// Vertex AI 后端会优先使用 `:fetchPredictOperation` 轮询视频生成操作。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或响应解析失败时返回错误。
    pub async fn get_generate_videos_operation(
        &self,
        operation: GenerateVideosOperation,
    ) -> Result<GenerateVideosOperation> {
        self.get_generate_videos_operation_with_config(operation, GetOperationConfig::default())
            .await
    }

    /// 获取 `GenerateVideos` 操作状态（带配置）。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或响应解析失败时返回错误。
    pub async fn get_generate_videos_operation_with_config(
        &self,
        operation: GenerateVideosOperation,
        mut config: GetOperationConfig,
    ) -> Result<GenerateVideosOperation> {
        let http_options = config.http_options.take();
        let backend = self.inner.config.backend;
        let name = operation.name.ok_or_else(|| Error::InvalidConfig {
            message: "Operation name is empty".into(),
        })?;

        let value = match backend {
            Backend::GeminiApi => self.get_operation_value(&name, http_options.as_ref()).await?,
            Backend::VertexAi => {
                // Video generation LROs are polled via `:fetchPredictOperation`.
                let resource_name = name
                    .rsplit_once("/operations/")
                    .map(|(resource, _)| resource)
                    .filter(|resource| resource.contains("/models/"));
                if let Some(resource_name) = resource_name {
                    self.fetch_predict_operation_value(&name, resource_name, http_options.as_ref())
                        .await?
                } else {
                    self.get_operation_value(&name, http_options.as_ref()).await?
                }
            }
        };

        crate::models::parsers::parse_generate_videos_operation(value, backend)
    }

    /// 等待 `GenerateVideos` 操作完成（轮询）。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或轮询过程中响应解析失败时返回错误。
    pub async fn wait_generate_videos_operation(
        &self,
        mut operation: GenerateVideosOperation,
    ) -> Result<GenerateVideosOperation> {
        let name = operation.name.clone().ok_or_else(|| Error::InvalidConfig {
            message: "Operation name is empty".into(),
        })?;
        while !operation.done.unwrap_or(false) {
            tokio::time::sleep(Duration::from_secs(5)).await;
            operation = self
                .get_generate_videos_operation(GenerateVideosOperation {
                    name: Some(name.clone()),
                    ..Default::default()
                })
                .await?;
        }
        Ok(operation)
    }

    /// 获取上传到 FileSearchStore 的操作状态（Gemini API only）。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或响应解析失败时返回错误。
    pub async fn get_upload_to_file_search_store_operation(
        &self,
        operation: UploadToFileSearchStoreOperation,
    ) -> Result<UploadToFileSearchStoreOperation> {
        self.get_upload_to_file_search_store_operation_with_config(
            operation,
            GetOperationConfig::default(),
        )
        .await
    }

    /// 获取上传到 FileSearchStore 的操作状态（带配置，Gemini API only）。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或响应解析失败时返回错误。
    pub async fn get_upload_to_file_search_store_operation_with_config(
        &self,
        operation: UploadToFileSearchStoreOperation,
        mut config: GetOperationConfig,
    ) -> Result<UploadToFileSearchStoreOperation> {
        if self.inner.config.backend == Backend::VertexAi {
            return Err(Error::InvalidConfig {
                message: "UploadToFileSearchStoreOperation is only supported in Gemini API"
                    .to_string(),
            });
        }
        let http_options = config.http_options.take();
        let name = operation.name.ok_or_else(|| Error::InvalidConfig {
            message: "Operation name is empty".into(),
        })?;
        let value = self.get_operation_value(&name, http_options.as_ref()).await?;
        Ok(serde_json::from_value(value)?)
    }

    /// 等待上传到 FileSearchStore 的操作完成（轮询，Gemini API only）。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或轮询过程中响应解析失败时返回错误。
    pub async fn wait_upload_to_file_search_store_operation(
        &self,
        mut operation: UploadToFileSearchStoreOperation,
    ) -> Result<UploadToFileSearchStoreOperation> {
        let name = operation.name.clone().ok_or_else(|| Error::InvalidConfig {
            message: "Operation name is empty".into(),
        })?;
        while !operation.done.unwrap_or(false) {
            tokio::time::sleep(Duration::from_secs(5)).await;
            operation = self
                .get_upload_to_file_search_store_operation(UploadToFileSearchStoreOperation {
                    name: Some(name.clone()),
                    ..Default::default()
                })
                .await?;
        }
        Ok(operation)
    }

    /// 获取导入文件到 FileSearchStore 的操作状态（Gemini API only）。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或响应解析失败时返回错误。
    pub async fn get_import_file_operation(
        &self,
        operation: ImportFileOperation,
    ) -> Result<ImportFileOperation> {
        self.get_import_file_operation_with_config(operation, GetOperationConfig::default())
            .await
    }

    /// 获取导入文件到 FileSearchStore 的操作状态（带配置，Gemini API only）。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或响应解析失败时返回错误。
    pub async fn get_import_file_operation_with_config(
        &self,
        operation: ImportFileOperation,
        mut config: GetOperationConfig,
    ) -> Result<ImportFileOperation> {
        if self.inner.config.backend == Backend::VertexAi {
            return Err(Error::InvalidConfig {
                message: "ImportFileOperation is only supported in Gemini API".to_string(),
            });
        }
        let http_options = config.http_options.take();
        let name = operation.name.ok_or_else(|| Error::InvalidConfig {
            message: "Operation name is empty".into(),
        })?;
        let value = self.get_operation_value(&name, http_options.as_ref()).await?;
        Ok(serde_json::from_value(value)?)
    }

    /// 等待导入文件到 FileSearchStore 的操作完成（轮询，Gemini API only）。
    ///
    /// # Errors
    /// 当请求失败、操作缺少名称或轮询过程中响应解析失败时返回错误。
    pub async fn wait_import_file_operation(
        &self,
        mut operation: ImportFileOperation,
    ) -> Result<ImportFileOperation> {
        let name = operation.name.clone().ok_or_else(|| Error::InvalidConfig {
            message: "Operation name is empty".into(),
        })?;
        while !operation.done.unwrap_or(false) {
            tokio::time::sleep(Duration::from_secs(5)).await;
            operation = self
                .get_import_file_operation(ImportFileOperation {
                    name: Some(name.clone()),
                    ..Default::default()
                })
                .await?;
        }
        Ok(operation)
    }
}

fn normalize_operation_name(inner: &ClientInner, name: &str) -> Result<String> {
    match inner.config.backend {
        Backend::GeminiApi => {
            // Gemini API may return LRO names under different resources
            // (e.g. `fileSearchStores/*/operations/*`, `tunedModels/*/operations/*`).
            // If the caller passes a full resource name, use it as-is.
            if name.contains('/') {
                Ok(name.to_string())
            } else {
                Ok(format!("operations/{name}"))
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
            } else if name.starts_with("operations/") {
                Ok(format!(
                    "projects/{}/locations/{}/{}",
                    vertex.project, vertex.location, name
                ))
            } else {
                Ok(format!(
                    "projects/{}/locations/{}/operations/{}",
                    vertex.project, vertex.location, name
                ))
            }
        }
    }
}

fn build_operation_url(
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

async fn fetch_predict_operation_value(
    inner: &Arc<ClientInner>,
    operation_name: &str,
    resource_name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<Value> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    let url = format!("{base}{version}/{resource_name}:fetchPredictOperation");

    let mut request = inner
        .http
        .post(url)
        .json(&serde_json::json!({ "operationName": operation_name }));
    request = apply_http_options(request, http_options)?;

    let response = inner.send_with_http_options(request, http_options).await?;
    if !response.status().is_success() {
        return Err(Error::ApiError {
            status: response.status().as_u16(),
            message: response.text().await.unwrap_or_default(),
        });
    }
    Ok(response.json::<Value>().await?)
}

async fn get_operation_value(
    inner: &Arc<ClientInner>,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<Value> {
    let name = normalize_operation_name(inner, name)?;
    let url = build_operation_url(inner, &name, http_options);
    let mut request = inner.http.get(url);
    request = apply_http_options(request, http_options)?;

    let response = inner.send_with_http_options(request, http_options).await?;
    if !response.status().is_success() {
        return Err(Error::ApiError {
            status: response.status().as_u16(),
            message: response.text().await.unwrap_or_default(),
        });
    }
    Ok(response.json::<Value>().await?)
}

impl Operations {
    async fn fetch_predict_operation_value(
        &self,
        operation_name: &str,
        resource_name: &str,
        http_options: Option<&rust_genai_types::http::HttpOptions>,
    ) -> Result<Value> {
        fetch_predict_operation_value(&self.inner, operation_name, resource_name, http_options).await
    }

    async fn get_operation_value(
        &self,
        name: &str,
        http_options: Option<&rust_genai_types::http::HttpOptions>,
    ) -> Result<Value> {
        get_operation_value(&self.inner, name, http_options).await
    }
}

fn build_operations_list_url(
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
        Backend::GeminiApi => format!("{base}{version}/operations"),
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
                "{base}{version}/projects/{}/locations/{}/operations",
                vertex.project, vertex.location
            )
        }
    };
    Ok(url)
}

fn add_list_query_params(url: &str, config: &ListOperationsConfig) -> Result<String> {
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
    use crate::test_support::{test_client_inner, test_vertex_inner_missing_config};
    use std::collections::HashMap;

    #[test]
    fn test_normalize_operation_name() {
        let gemini = test_client_inner(Backend::GeminiApi);
        assert_eq!(
            normalize_operation_name(&gemini, "operations/123").unwrap(),
            "operations/123"
        );
        assert_eq!(
            normalize_operation_name(&gemini, "models/abc").unwrap(),
            "models/abc"
        );
        assert_eq!(
            normalize_operation_name(&gemini, "fileSearchStores/s/operations/o").unwrap(),
            "fileSearchStores/s/operations/o"
        );
        assert_eq!(
            normalize_operation_name(&gemini, "op-1").unwrap(),
            "operations/op-1"
        );

        let vertex = test_client_inner(Backend::VertexAi);
        assert_eq!(
            normalize_operation_name(&vertex, "projects/x/locations/y/operations/z").unwrap(),
            "projects/x/locations/y/operations/z"
        );
        assert_eq!(
            normalize_operation_name(&vertex, "locations/us/operations/1").unwrap(),
            "projects/proj/locations/us/operations/1"
        );
        assert_eq!(
            normalize_operation_name(&vertex, "operations/2").unwrap(),
            "projects/proj/locations/loc/operations/2"
        );
        assert_eq!(
            normalize_operation_name(&vertex, "op-3").unwrap(),
            "projects/proj/locations/loc/operations/op-3"
        );
    }

    #[test]
    fn test_build_operations_list_url_and_params() {
        let gemini = test_client_inner(Backend::GeminiApi);
        let url = build_operations_list_url(&gemini, None).unwrap();
        assert!(url.ends_with("/v1beta/operations"));
        let url = add_list_query_params(
            &url,
            &ListOperationsConfig {
                page_size: Some(10),
                page_token: Some("token".to_string()),
                filter: Some("done=true".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(url.contains("pageSize=10"));
        assert!(url.contains("pageToken=token"));

        let vertex = test_client_inner(Backend::VertexAi);
        let url = build_operations_list_url(&vertex, None).unwrap();
        assert!(url.contains("/projects/proj/locations/loc/operations"));
    }

    #[test]
    fn test_build_operations_list_url_vertex_missing_config_errors() {
        let inner = test_vertex_inner_missing_config();
        assert!(build_operations_list_url(&inner, None).is_err());
    }

    #[test]
    fn test_add_list_query_params_invalid_url() {
        let err = add_list_query_params("::bad", &ListOperationsConfig::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_apply_http_options_invalid_header() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = rust_genai_types::http::HttpOptions {
            headers: Some([("bad header".to_string(), "value".to_string())].into()),
            ..Default::default()
        };
        let err = apply_http_options(request, Some(&options)).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_apply_http_options_with_valid_header() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let mut headers = HashMap::new();
        headers.insert("x-test".to_string(), "ok".to_string());
        let options = rust_genai_types::http::HttpOptions {
            headers: Some(headers),
            ..Default::default()
        };
        let request = apply_http_options(request, Some(&options)).unwrap();
        let built = request.build().unwrap();
        assert!(built.headers().contains_key("x-test"));
    }

    #[test]
    fn test_apply_http_options_invalid_header_value() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let mut headers = HashMap::new();
        headers.insert("x-test".to_string(), "bad\nvalue".to_string());
        let options = rust_genai_types::http::HttpOptions {
            headers: Some(headers),
            ..Default::default()
        };
        let err = apply_http_options(request, Some(&options)).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[tokio::test]
    async fn test_wait_missing_name_errors() {
        let client = crate::Client::new("test-key").unwrap();
        let ops = client.operations();
        let result = ops
            .wait(Operation {
                name: None,
                done: Some(false),
                ..Default::default()
            })
            .await;
        assert!(matches!(result.unwrap_err(), Error::InvalidConfig { .. }));
    }
}
