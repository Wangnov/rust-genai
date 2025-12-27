//! Operations API surface.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::operations::{
    GetOperationConfig, ListOperationsConfig, ListOperationsResponse, Operation,
};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct Operations {
    pub(crate) inner: Arc<ClientInner>,
}

impl Operations {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 获取操作状态。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<Operation> {
        self.get_with_config(name, GetOperationConfig::default())
            .await
    }

    /// 获取操作状态（带配置）。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetOperationConfig,
    ) -> Result<Operation> {
        let http_options = config.http_options.take();
        let name = normalize_operation_name(&self.inner, name.as_ref())?;
        let url = build_operation_url(&self.inner, &name, http_options.as_ref())?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<Operation>().await?)
    }

    /// 列出操作。
    pub async fn list(&self) -> Result<ListOperationsResponse> {
        self.list_with_config(ListOperationsConfig::default()).await
    }

    /// 列出操作（带配置）。
    pub async fn list_with_config(
        &self,
        mut config: ListOperationsConfig,
    ) -> Result<ListOperationsResponse> {
        let http_options = config.http_options.take();
        let url = build_operations_list_url(&self.inner, http_options.as_ref())?;
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
        Ok(response.json::<ListOperationsResponse>().await?)
    }

    /// 列出所有操作（自动翻页）。
    pub async fn all(&self) -> Result<Vec<Operation>> {
        self.all_with_config(ListOperationsConfig::default()).await
    }

    /// 列出所有操作（带配置，自动翻页）。
    pub async fn all_with_config(
        &self,
        mut config: ListOperationsConfig,
    ) -> Result<Vec<Operation>> {
        let mut ops = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options = http_options.clone();
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
}

fn normalize_operation_name(inner: &ClientInner, name: &str) -> Result<String> {
    match inner.config.backend {
        Backend::GeminiApi => {
            if name.starts_with("operations/") || name.starts_with("models/") {
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
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!("{base}{version}/{name}"))
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

fn add_list_query_params(url: String, config: &ListOperationsConfig) -> Result<String> {
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
