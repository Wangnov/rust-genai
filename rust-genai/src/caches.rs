//! Caches API surface.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::caches::{
    CachedContent, CreateCachedContentConfig, DeleteCachedContentConfig, GetCachedContentConfig,
    ListCachedContentsConfig, ListCachedContentsResponse, UpdateCachedContentConfig,
};
use serde_json::{json, Map, Value};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct Caches {
    pub(crate) inner: Arc<ClientInner>,
}

impl Caches {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建缓存。
    pub async fn create(
        &self,
        model: impl Into<String>,
        mut config: CreateCachedContentConfig,
    ) -> Result<CachedContent> {
        let http_options = config.http_options.take();
        let model = normalize_cache_model(&self.inner, &model.into())?;

        let mut body = serde_json::to_value(&config)?;
        let body_map = body.as_object_mut().ok_or_else(|| Error::Parse {
            message: "CreateCachedContentConfig must be object".into(),
        })?;
        body_map.insert("model".to_string(), Value::String(model));

        handle_kms_key(&self.inner, body_map)?;

        let mut body = Value::Object(body_map.clone());
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }

        let url = build_cached_contents_url(&self.inner, http_options.as_ref())?;
        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<CachedContent>().await?)
    }

    /// 获取缓存。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<CachedContent> {
        self.get_with_config(name, GetCachedContentConfig::default())
            .await
    }

    /// 获取缓存（带配置）。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetCachedContentConfig,
    ) -> Result<CachedContent> {
        let http_options = config.http_options.take();
        let name = normalize_cached_content_name(&self.inner, name.as_ref())?;
        let url = build_cached_content_url(&self.inner, &name, http_options.as_ref())?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<CachedContent>().await?)
    }

    /// 更新缓存（TTL/过期时间）。
    pub async fn update(
        &self,
        name: impl AsRef<str>,
        mut config: UpdateCachedContentConfig,
    ) -> Result<CachedContent> {
        let http_options = config.http_options.take();
        let name = normalize_cached_content_name(&self.inner, name.as_ref())?;
        let url = build_cached_content_url(&self.inner, &name, http_options.as_ref())?;
        let mut body = serde_json::to_value(&config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let mut request = self.inner.http.patch(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<CachedContent>().await?)
    }

    /// 删除缓存。
    pub async fn delete(&self, name: impl AsRef<str>) -> Result<()> {
        self.delete_with_config(name, DeleteCachedContentConfig::default())
            .await
    }

    /// 删除缓存（带配置）。
    pub async fn delete_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: DeleteCachedContentConfig,
    ) -> Result<()> {
        let http_options = config.http_options.take();
        let name = normalize_cached_content_name(&self.inner, name.as_ref())?;
        let url = build_cached_content_url(&self.inner, &name, http_options.as_ref())?;
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

    /// 列出缓存。
    pub async fn list(&self) -> Result<ListCachedContentsResponse> {
        self.list_with_config(ListCachedContentsConfig::default())
            .await
    }

    /// 列出缓存（带配置）。
    pub async fn list_with_config(
        &self,
        mut config: ListCachedContentsConfig,
    ) -> Result<ListCachedContentsResponse> {
        let http_options = config.http_options.take();
        let url = build_cached_contents_url(&self.inner, http_options.as_ref())?;
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
        Ok(response.json::<ListCachedContentsResponse>().await?)
    }

    /// 列出所有缓存（自动翻页）。
    pub async fn all(&self) -> Result<Vec<CachedContent>> {
        self.all_with_config(ListCachedContentsConfig::default())
            .await
    }

    /// 列出所有缓存（带配置，自动翻页）。
    pub async fn all_with_config(
        &self,
        mut config: ListCachedContentsConfig,
    ) -> Result<Vec<CachedContent>> {
        let mut contents = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options = http_options.clone();
            let response = self.list_with_config(page_config).await?;
            if let Some(items) = response.cached_contents {
                contents.extend(items);
            }
            match response.next_page_token {
                Some(token) if !token.is_empty() => {
                    config.page_token = Some(token);
                }
                _ => break,
            }
        }
        Ok(contents)
    }
}

fn normalize_cache_model(inner: &ClientInner, model: &str) -> Result<String> {
    match inner.config.backend {
        Backend::GeminiApi => {
            if model.starts_with("models/") || model.starts_with("tunedModels/") {
                Ok(model.to_string())
            } else {
                Ok(format!("models/{model}"))
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
            if model.starts_with("projects/") {
                Ok(model.to_string())
            } else if model.starts_with("publishers/") {
                Ok(format!(
                    "projects/{}/locations/{}/{}",
                    vertex.project, vertex.location, model
                ))
            } else if model.starts_with("models/") {
                Ok(format!(
                    "projects/{}/locations/{}/publishers/google/{}",
                    vertex.project, vertex.location, model
                ))
            } else if let Some((publisher, name)) = model.split_once('/') {
                Ok(format!(
                    "projects/{}/locations/{}/publishers/{}/models/{}",
                    vertex.project, vertex.location, publisher, name
                ))
            } else {
                Ok(format!(
                    "projects/{}/locations/{}/publishers/google/models/{}",
                    vertex.project, vertex.location, model
                ))
            }
        }
    }
}

fn normalize_cached_content_name(inner: &ClientInner, name: &str) -> Result<String> {
    match inner.config.backend {
        Backend::GeminiApi => {
            if name.starts_with("cachedContents/") {
                Ok(name.to_string())
            } else {
                Ok(format!("cachedContents/{name}"))
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
            } else if name.starts_with("cachedContents/") {
                Ok(format!(
                    "projects/{}/locations/{}/{}",
                    vertex.project, vertex.location, name
                ))
            } else {
                Ok(format!(
                    "projects/{}/locations/{}/cachedContents/{}",
                    vertex.project, vertex.location, name
                ))
            }
        }
    }
}

fn build_cached_contents_url(
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
        Backend::GeminiApi => format!("{base}{version}/cachedContents"),
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
                "{base}{version}/projects/{}/locations/{}/cachedContents",
                vertex.project, vertex.location
            )
        }
    };
    Ok(url)
}

fn build_cached_content_url(
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

fn add_list_query_params(url: String, config: &ListCachedContentsConfig) -> Result<String> {
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

fn handle_kms_key(inner: &ClientInner, body: &mut Map<String, Value>) -> Result<()> {
    let kms_key_name = body.remove("kmsKeyName");
    if let Some(kms_key_name) = kms_key_name {
        match inner.config.backend {
            Backend::GeminiApi => {
                return Err(Error::InvalidConfig {
                    message: "kms_key_name is not supported in Gemini API".into(),
                })
            }
            Backend::VertexAi => {
                body.insert(
                    "encryptionSpec".to_string(),
                    json!({ "kmsKeyName": kms_key_name }),
                );
            }
        }
    }
    Ok(())
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
    use crate::client::{ApiClient, Backend, ClientConfig, ClientInner, Credentials, HttpOptions};

    #[test]
    fn test_normalize_cached_content_name_gemini() {
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
        let name = normalize_cached_content_name(&inner, "abc-123").unwrap();
        assert_eq!(name, "cachedContents/abc-123");
    }
}
