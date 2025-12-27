//! Documents API surface.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::documents::{
    DeleteDocumentConfig, Document, GetDocumentConfig, ListDocumentsConfig, ListDocumentsResponse,
};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct Documents {
    pub(crate) inner: Arc<ClientInner>,
}

impl Documents {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 获取 Document。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<Document> {
        self.get_with_config(name, GetDocumentConfig::default())
            .await
    }

    /// 获取 Document（带配置）。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetDocumentConfig,
    ) -> Result<Document> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_document_name(name.as_ref())?;
        let url = build_document_url(&self.inner, &name, http_options.as_ref())?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<Document>().await?)
    }

    /// 删除 Document。
    pub async fn delete(&self, name: impl AsRef<str>) -> Result<()> {
        self.delete_with_config(name, DeleteDocumentConfig::default())
            .await
    }

    /// 删除 Document（带配置）。
    pub async fn delete_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: DeleteDocumentConfig,
    ) -> Result<()> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_document_name(name.as_ref())?;
        let url = build_document_url(&self.inner, &name, http_options.as_ref())?;
        let url = add_delete_query_params(url, &config)?;
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

    /// 列出 Documents。
    pub async fn list(&self, parent: impl AsRef<str>) -> Result<ListDocumentsResponse> {
        self.list_with_config(parent, ListDocumentsConfig::default())
            .await
    }

    /// 列出 Documents（带配置）。
    pub async fn list_with_config(
        &self,
        parent: impl AsRef<str>,
        mut config: ListDocumentsConfig,
    ) -> Result<ListDocumentsResponse> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let parent = normalize_file_search_store_name(parent.as_ref());
        let url = build_documents_url(&self.inner, &parent, http_options.as_ref())?;
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
        Ok(response.json::<ListDocumentsResponse>().await?)
    }

    /// 列出所有 Documents（自动翻页）。
    pub async fn all(&self, parent: impl AsRef<str>) -> Result<Vec<Document>> {
        self.all_with_config(parent, ListDocumentsConfig::default())
            .await
    }

    /// 列出所有 Documents（带配置，自动翻页）。
    pub async fn all_with_config(
        &self,
        parent: impl AsRef<str>,
        mut config: ListDocumentsConfig,
    ) -> Result<Vec<Document>> {
        let mut docs = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options = http_options.clone();
            let response = self.list_with_config(parent.as_ref(), page_config).await?;
            if let Some(items) = response.documents {
                docs.extend(items);
            }
            match response.next_page_token {
                Some(token) if !token.is_empty() => {
                    config.page_token = Some(token);
                }
                _ => break,
            }
        }
        Ok(docs)
    }
}

fn ensure_gemini_backend(inner: &ClientInner) -> Result<()> {
    if inner.config.backend == Backend::VertexAi {
        return Err(Error::InvalidConfig {
            message: "Documents API is only supported in Gemini API".into(),
        });
    }
    Ok(())
}

fn normalize_file_search_store_name(name: &str) -> String {
    if name.starts_with("fileSearchStores/") {
        name.to_string()
    } else {
        format!("fileSearchStores/{name}")
    }
}

fn normalize_document_name(name: &str) -> Result<String> {
    if name.contains("/documents/") {
        Ok(name.to_string())
    } else {
        Err(Error::InvalidConfig {
            message: format!(
                "Document name must be a full resource name, e.g. fileSearchStores/xxx/documents/yyy (got {name})"
            ),
        })
    }
}

fn build_document_url(
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

fn build_documents_url(
    inner: &ClientInner,
    parent: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!("{base}{version}/{parent}/documents"))
}

fn add_list_query_params(url: String, config: &ListDocumentsConfig) -> Result<String> {
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
    }
    Ok(url.to_string())
}

fn add_delete_query_params(url: String, config: &DeleteDocumentConfig) -> Result<String> {
    let mut url = reqwest::Url::parse(&url).map_err(|err| Error::InvalidConfig {
        message: err.to_string(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(force) = config.force {
            pairs.append_pair("force", &force.to_string());
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
