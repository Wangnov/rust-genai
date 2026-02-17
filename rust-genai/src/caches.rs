//! Caches API surface.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::caches::{
    CachedContent, CreateCachedContentConfig, DeleteCachedContentConfig, DeleteCachedContentResponse,
    GetCachedContentConfig, ListCachedContentsConfig, ListCachedContentsResponse,
    UpdateCachedContentConfig,
};
use serde_json::{json, Map, Value};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};
use crate::http_response::sdk_http_response_from_headers;

#[derive(Clone)]
pub struct Caches {
    pub(crate) inner: Arc<ClientInner>,
}

impl Caches {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建缓存。
    ///
    /// # Errors
    /// 当请求失败、服务端返回错误或响应解析失败时返回错误。
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
        Ok(response.json::<CachedContent>().await?)
    }

    /// 获取缓存。
    ///
    /// # Errors
    /// 当请求失败、服务端返回错误或响应解析失败时返回错误。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<CachedContent> {
        self.get_with_config(name, GetCachedContentConfig::default())
            .await
    }

    /// 获取缓存（带配置）。
    ///
    /// # Errors
    /// 当请求失败、服务端返回错误或响应解析失败时返回错误。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetCachedContentConfig,
    ) -> Result<CachedContent> {
        let http_options = config.http_options.take();
        let name = normalize_cached_content_name(&self.inner, name.as_ref())?;
        let url = build_cached_content_url(&self.inner, &name, http_options.as_ref());
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
        Ok(response.json::<CachedContent>().await?)
    }

    /// 更新缓存（TTL/过期时间）。
    ///
    /// # Errors
    /// 当请求失败、服务端返回错误或响应解析失败时返回错误。
    pub async fn update(
        &self,
        name: impl AsRef<str>,
        mut config: UpdateCachedContentConfig,
    ) -> Result<CachedContent> {
        let http_options = config.http_options.take();
        let name = normalize_cached_content_name(&self.inner, name.as_ref())?;
        let url = build_cached_content_url(&self.inner, &name, http_options.as_ref());
        let mut body = serde_json::to_value(&config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let mut request = self.inner.http.patch(url).json(&body);
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
        Ok(response.json::<CachedContent>().await?)
    }

    /// 删除缓存。
    ///
    /// # Errors
    /// 当请求失败或服务端返回错误时返回错误。
    pub async fn delete(&self, name: impl AsRef<str>) -> Result<DeleteCachedContentResponse> {
        self.delete_with_config(name, DeleteCachedContentConfig::default())
            .await
    }

    /// 删除缓存（带配置）。
    ///
    /// # Errors
    /// 当请求失败或服务端返回错误时返回错误。
    pub async fn delete_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: DeleteCachedContentConfig,
    ) -> Result<DeleteCachedContentResponse> {
        let http_options = config.http_options.take();
        let name = normalize_cached_content_name(&self.inner, name.as_ref())?;
        let url = build_cached_content_url(&self.inner, &name, http_options.as_ref());
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
            DeleteCachedContentResponse::default()
        } else {
            serde_json::from_str::<DeleteCachedContentResponse>(&text)?
        };
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 列出缓存。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list(&self) -> Result<ListCachedContentsResponse> {
        self.list_with_config(ListCachedContentsConfig::default())
            .await
    }

    /// 列出缓存（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list_with_config(
        &self,
        mut config: ListCachedContentsConfig,
    ) -> Result<ListCachedContentsResponse> {
        let http_options = config.http_options.take();
        let url = build_cached_contents_url(&self.inner, http_options.as_ref())?;
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
        let headers = response.headers().clone();
        let mut result = response.json::<ListCachedContentsResponse>().await?;
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 列出所有缓存（自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all(&self) -> Result<Vec<CachedContent>> {
        self.all_with_config(ListCachedContentsConfig::default())
            .await
    }

    /// 列出所有缓存（带配置，自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all_with_config(
        &self,
        mut config: ListCachedContentsConfig,
    ) -> Result<Vec<CachedContent>> {
        let mut contents = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options.clone_from(&http_options);
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
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/{name}")
}

fn add_list_query_params(url: &str, config: &ListCachedContentsConfig) -> Result<String> {
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
    use crate::test_support::{
        test_client_inner, test_client_inner_with_base, test_vertex_inner_missing_config,
    };
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param, query_param_is_missing};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_normalize_cached_content_name_gemini() {
        let inner = test_client_inner(Backend::GeminiApi);
        let name = normalize_cached_content_name(&inner, "abc-123").unwrap();
        assert_eq!(name, "cachedContents/abc-123");
        let name = normalize_cache_model(&inner, "models/gemini-1.5-pro").unwrap();
        assert_eq!(name, "models/gemini-1.5-pro");
    }

    #[test]
    fn test_normalize_cached_content_name_vertex_and_model() {
        let inner = test_client_inner(Backend::VertexAi);
        assert_eq!(
            normalize_cached_content_name(&inner, "cachedContents/1").unwrap(),
            "projects/proj/locations/loc/cachedContents/1"
        );
        assert_eq!(
            normalize_cached_content_name(&inner, "locations/us/cachedContents/1").unwrap(),
            "projects/proj/locations/us/cachedContents/1"
        );
        assert_eq!(
            normalize_cached_content_name(&inner, "projects/p/locations/l/cachedContents/1")
                .unwrap(),
            "projects/p/locations/l/cachedContents/1"
        );
        assert_eq!(
            normalize_cached_content_name(&inner, "custom-id").unwrap(),
            "projects/proj/locations/loc/cachedContents/custom-id"
        );
        assert_eq!(
            normalize_cache_model(&inner, "gemini-1.5-pro").unwrap(),
            "projects/proj/locations/loc/publishers/google/models/gemini-1.5-pro"
        );
    }

    #[test]
    fn test_normalize_cache_model_vertex_variants() {
        let inner = test_client_inner(Backend::VertexAi);
        assert_eq!(
            normalize_cache_model(&inner, "projects/p/locations/l/publishers/google/models/m1")
                .unwrap(),
            "projects/p/locations/l/publishers/google/models/m1"
        );
        assert_eq!(
            normalize_cache_model(&inner, "publishers/google/models/m2").unwrap(),
            "projects/proj/locations/loc/publishers/google/models/m2"
        );
        assert_eq!(
            normalize_cache_model(&inner, "models/m3").unwrap(),
            "projects/proj/locations/loc/publishers/google/models/m3"
        );
        assert_eq!(
            normalize_cache_model(&inner, "custom/m4").unwrap(),
            "projects/proj/locations/loc/publishers/custom/models/m4"
        );
    }

    #[test]
    fn test_vertex_missing_config_errors() {
        let inner = test_vertex_inner_missing_config();
        assert!(normalize_cache_model(&inner, "gemini-1.5-pro").is_err());
        assert!(normalize_cached_content_name(&inner, "cachedContents/1").is_err());
        assert!(build_cached_contents_url(&inner, None).is_err());
    }

    #[test]
    fn test_handle_kms_key_and_query_params() {
        let gemini = test_client_inner(Backend::GeminiApi);
        let mut body = json!({"kmsKeyName": "key"}).as_object().unwrap().clone();
        let err = handle_kms_key(&gemini, &mut body).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let vertex = test_client_inner(Backend::VertexAi);
        let mut body = json!({"kmsKeyName": "key"}).as_object().unwrap().clone();
        handle_kms_key(&vertex, &mut body).unwrap();
        assert!(body.get("encryptionSpec").is_some());

        let url = add_list_query_params(
            "https://example.com/cachedContents",
            &ListCachedContentsConfig {
                page_size: Some(2),
                page_token: Some("t".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(url.contains("pageSize=2"));
        assert!(url.contains("pageToken=t"));
    }

    #[test]
    fn test_add_list_query_params_invalid_url() {
        let err =
            add_list_query_params("not a url", &ListCachedContentsConfig::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_apply_http_options_invalid_header() {
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
    fn test_apply_http_options_with_timeout_and_headers() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = rust_genai_types::http::HttpOptions {
            timeout: Some(1500),
            headers: Some([("x-test".to_string(), "value".to_string())].into()),
            ..Default::default()
        };
        let request = apply_http_options(request, Some(&options))
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(request.headers().get("x-test").unwrap(), "value");
    }

    #[test]
    fn test_build_cache_urls_and_merge_extra_body() {
        let gemini = test_client_inner(Backend::GeminiApi);
        let url = build_cached_contents_url(&gemini, None).unwrap();
        assert!(url.ends_with("/v1beta/cachedContents"));
        let url = build_cached_content_url(&gemini, "cachedContents/1", None);
        assert!(url.ends_with("/v1beta/cachedContents/1"));

        let vertex = test_client_inner(Backend::VertexAi);
        let url = build_cached_contents_url(&vertex, None).unwrap();
        assert!(url.contains("/projects/proj/locations/loc/cachedContents"));

        let mut body = serde_json::json!({"a": 1});
        let options = rust_genai_types::http::HttpOptions {
            extra_body: Some(serde_json::json!({"b": 2})),
            ..Default::default()
        };
        merge_extra_body(&mut body, &options).unwrap();
        assert_eq!(body.get("b").and_then(serde_json::Value::as_i64), Some(2));
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
    fn test_merge_extra_body_invalid() {
        let mut body = serde_json::json!({});
        let options = rust_genai_types::http::HttpOptions {
            extra_body: Some(serde_json::json!("bad")),
            ..Default::default()
        };
        let err = merge_extra_body(&mut body, &options).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[tokio::test]
    async fn test_create_update_with_extra_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1beta/cachedContents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "cachedContents/1"
            })))
            .mount(&server)
            .await;
        Mock::given(method("PATCH"))
            .and(path("/v1beta/cachedContents/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "cachedContents/1"
            })))
            .mount(&server)
            .await;

        let inner = test_client_inner_with_base(Backend::GeminiApi, &server.uri(), "v1beta");
        let caches = Caches::new(Arc::new(inner));
        let create = CreateCachedContentConfig {
            http_options: Some(rust_genai_types::http::HttpOptions {
                extra_body: Some(json!({"extra": "value"})),
                ..Default::default()
            }),
            ..Default::default()
        };
        let created = caches
            .create("models/gemini-1.5-pro", create)
            .await
            .unwrap();
        assert_eq!(created.name.as_deref(), Some("cachedContents/1"));

        let update = UpdateCachedContentConfig {
            http_options: Some(rust_genai_types::http::HttpOptions {
                extra_body: Some(json!({"extra": "value"})),
                ..Default::default()
            }),
            ..Default::default()
        };
        let updated = caches.update("cachedContents/1", update).await.unwrap();
        assert_eq!(updated.name.as_deref(), Some("cachedContents/1"));
    }

    #[tokio::test]
    async fn test_caches_vertex_api_flow() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1beta1/projects/proj/locations/loc/cachedContents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "projects/proj/locations/loc/cachedContents/1"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/cachedContents/1",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "projects/proj/locations/loc/cachedContents/1"
            })))
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/cachedContents/1",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "projects/proj/locations/loc/cachedContents/1"
            })))
            .mount(&server)
            .await;

        Mock::given(method("DELETE"))
            .and(path(
                "/v1beta1/projects/proj/locations/loc/cachedContents/1",
            ))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v1beta1/projects/proj/locations/loc/cachedContents"))
            .and(query_param("pageSize", "2"))
            .and(query_param_is_missing("pageToken"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "cachedContents": [{"name": "projects/proj/locations/loc/cachedContents/1"}],
                "nextPageToken": "next"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v1beta1/projects/proj/locations/loc/cachedContents"))
            .and(query_param("pageSize", "2"))
            .and(query_param("pageToken", "next"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "cachedContents": [{"name": "projects/proj/locations/loc/cachedContents/2"}]
            })))
            .mount(&server)
            .await;

        let inner = test_client_inner_with_base(Backend::VertexAi, &server.uri(), "v1beta1");
        let caches = Caches::new(Arc::new(inner));

        let created = caches
            .create("gemini-1.5-pro", CreateCachedContentConfig::default())
            .await
            .unwrap();
        assert!(created
            .name
            .as_deref()
            .unwrap()
            .contains("cachedContents/1"));

        let got = caches.get("cachedContents/1").await.unwrap();
        assert!(got.name.as_deref().unwrap().contains("cachedContents/1"));

        let updated = caches
            .update("cachedContents/1", UpdateCachedContentConfig::default())
            .await
            .unwrap();
        assert!(updated
            .name
            .as_deref()
            .unwrap()
            .contains("cachedContents/1"));

        caches.delete("cachedContents/1").await.unwrap();

        let list = caches
            .list_with_config(ListCachedContentsConfig {
                page_size: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(list.cached_contents.unwrap().len(), 1);

        let all = caches
            .all_with_config(ListCachedContentsConfig {
                page_size: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
    }
}
