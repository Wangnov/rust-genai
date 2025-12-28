//! Client configuration and transport layer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use reqwest::{Client as HttpClient, Proxy};
use tokio::sync::OnceCell;

use crate::auth::OAuthTokenProvider;
use crate::error::{Error, Result};
use google_cloud_auth::credentials::{
    Builder as AuthBuilder, CacheableResource, Credentials as GoogleCredentials,
};
use http::Extensions;

/// Gemini 客户端。
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

pub(crate) struct ClientInner {
    pub http: HttpClient,
    pub config: ClientConfig,
    pub api_client: ApiClient,
    pub(crate) auth_provider: Option<AuthProvider>,
}

/// 客户端配置。
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// API 密钥（Gemini API）。
    pub api_key: Option<String>,
    /// 后端选择。
    pub backend: Backend,
    /// Vertex AI 配置。
    pub vertex_config: Option<VertexConfig>,
    /// HTTP 配置。
    pub http_options: HttpOptions,
    /// 认证信息。
    pub credentials: Credentials,
    /// OAuth scopes（服务账号/ADC 使用）。
    pub auth_scopes: Vec<String>,
}

/// 后端选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    GeminiApi,
    VertexAi,
}

/// 认证方式。
#[derive(Debug, Clone)]
pub enum Credentials {
    /// API Key（Gemini API）。
    ApiKey(String),
    /// OAuth 用户凭据。
    OAuth {
        client_secret_path: PathBuf,
        token_cache_path: Option<PathBuf>,
    },
    /// Application Default Credentials (ADC)。
    ApplicationDefault,
}

/// Vertex AI 配置。
#[derive(Debug, Clone)]
pub struct VertexConfig {
    pub project: String,
    pub location: String,
    pub credentials: Option<VertexCredentials>,
}

/// Vertex AI 认证占位。
#[derive(Debug, Clone)]
pub struct VertexCredentials {
    pub access_token: Option<String>,
}

/// HTTP 配置。
#[derive(Debug, Clone, Default)]
pub struct HttpOptions {
    pub timeout: Option<u64>,
    pub proxy: Option<String>,
    pub headers: HashMap<String, String>,
    pub base_url: Option<String>,
    pub api_version: Option<String>,
}

impl Client {
    /// 创建新客户端（Gemini API）。
    ///
    /// # Errors
    /// 当配置无效或构建客户端失败时返回错误。
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::builder()
            .api_key(api_key)
            .backend(Backend::GeminiApi)
            .build()
    }

    /// 从环境变量创建客户端。
    ///
    /// # Errors
    /// 当环境变量缺失或构建客户端失败时返回错误。
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .map_err(|_| Error::InvalidConfig {
                message: "GEMINI_API_KEY or GOOGLE_API_KEY not found".into(),
            })?;
        let mut builder = Self::builder().api_key(api_key);
        if let Ok(base_url) =
            std::env::var("GENAI_BASE_URL").or_else(|_| std::env::var("GEMINI_BASE_URL"))
        {
            if !base_url.trim().is_empty() {
                builder = builder.base_url(base_url);
            }
        }
        if let Ok(api_version) = std::env::var("GENAI_API_VERSION") {
            if !api_version.trim().is_empty() {
                builder = builder.api_version(api_version);
            }
        }
        builder.build()
    }

    /// 创建 Vertex AI 客户端。
    ///
    /// # Errors
    /// 当配置无效或构建客户端失败时返回错误。
    pub fn new_vertex(project: impl Into<String>, location: impl Into<String>) -> Result<Self> {
        Self::builder()
            .backend(Backend::VertexAi)
            .vertex_project(project)
            .vertex_location(location)
            .build()
    }

    /// 使用 OAuth 凭据创建客户端（默认读取 token.json）。
    ///
    /// # Errors
    /// 当凭据路径无效或构建客户端失败时返回错误。
    pub fn with_oauth(client_secret_path: impl AsRef<Path>) -> Result<Self> {
        Self::builder()
            .credentials(Credentials::OAuth {
                client_secret_path: client_secret_path.as_ref().to_path_buf(),
                token_cache_path: None,
            })
            .build()
    }

    /// 使用 Application Default Credentials 创建客户端。
    ///
    /// # Errors
    /// 当构建客户端失败时返回错误。
    pub fn with_adc() -> Result<Self> {
        Self::builder()
            .credentials(Credentials::ApplicationDefault)
            .build()
    }

    /// 创建 Builder。
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// 访问 Models API。
    #[must_use]
    pub fn models(&self) -> crate::models::Models {
        crate::models::Models::new(self.inner.clone())
    }

    /// 访问 Chats API。
    #[must_use]
    pub fn chats(&self) -> crate::chats::Chats {
        crate::chats::Chats::new(self.inner.clone())
    }

    /// 访问 Files API。
    #[must_use]
    pub fn files(&self) -> crate::files::Files {
        crate::files::Files::new(self.inner.clone())
    }

    /// 访问 `FileSearchStores` API。
    #[must_use]
    pub fn file_search_stores(&self) -> crate::file_search_stores::FileSearchStores {
        crate::file_search_stores::FileSearchStores::new(self.inner.clone())
    }

    /// 访问 Documents API。
    #[must_use]
    pub fn documents(&self) -> crate::documents::Documents {
        crate::documents::Documents::new(self.inner.clone())
    }

    /// 访问 Live API。
    #[must_use]
    pub fn live(&self) -> crate::live::Live {
        crate::live::Live::new(self.inner.clone())
    }

    /// 访问 Live Music API。
    #[must_use]
    pub fn live_music(&self) -> crate::live_music::LiveMusic {
        crate::live_music::LiveMusic::new(self.inner.clone())
    }

    /// 访问 Caches API。
    #[must_use]
    pub fn caches(&self) -> crate::caches::Caches {
        crate::caches::Caches::new(self.inner.clone())
    }

    /// 访问 Batches API。
    #[must_use]
    pub fn batches(&self) -> crate::batches::Batches {
        crate::batches::Batches::new(self.inner.clone())
    }

    /// 访问 Tunings API。
    #[must_use]
    pub fn tunings(&self) -> crate::tunings::Tunings {
        crate::tunings::Tunings::new(self.inner.clone())
    }

    /// 访问 Operations API。
    #[must_use]
    pub fn operations(&self) -> crate::operations::Operations {
        crate::operations::Operations::new(self.inner.clone())
    }

    /// 访问 `AuthTokens` API（Ephemeral Tokens）。
    #[must_use]
    pub fn auth_tokens(&self) -> crate::tokens::AuthTokens {
        crate::tokens::AuthTokens::new(self.inner.clone())
    }

    /// 访问 Interactions API。
    #[must_use]
    pub fn interactions(&self) -> crate::interactions::Interactions {
        crate::interactions::Interactions::new(self.inner.clone())
    }

    /// 访问 Deep Research。
    #[must_use]
    pub fn deep_research(&self) -> crate::deep_research::DeepResearch {
        crate::deep_research::DeepResearch::new(self.inner.clone())
    }
}

/// 客户端 Builder。
#[derive(Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    credentials: Option<Credentials>,
    backend: Option<Backend>,
    vertex_project: Option<String>,
    vertex_location: Option<String>,
    http_options: HttpOptions,
    auth_scopes: Option<Vec<String>>,
}

impl ClientBuilder {
    /// 设置 API Key（Gemini API）。
    #[must_use]
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// 设置认证方式（OAuth/ADC/API Key）。
    #[must_use]
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// 设置后端（Gemini API 或 Vertex AI）。
    #[must_use]
    pub const fn backend(mut self, backend: Backend) -> Self {
        self.backend = Some(backend);
        self
    }

    /// 设置 Vertex AI 项目 ID。
    #[must_use]
    pub fn vertex_project(mut self, project: impl Into<String>) -> Self {
        self.vertex_project = Some(project.into());
        self
    }

    /// 设置 Vertex AI 区域。
    #[must_use]
    pub fn vertex_location(mut self, location: impl Into<String>) -> Self {
        self.vertex_location = Some(location.into());
        self
    }

    /// 设置请求超时（秒）。
    #[must_use]
    pub const fn timeout(mut self, secs: u64) -> Self {
        self.http_options.timeout = Some(secs);
        self
    }

    /// 设置代理。
    #[must_use]
    pub fn proxy(mut self, url: impl Into<String>) -> Self {
        self.http_options.proxy = Some(url.into());
        self
    }

    /// 增加默认 HTTP 头。
    #[must_use]
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.http_options.headers.insert(key.into(), value.into());
        self
    }

    /// 设置自定义基础 URL。
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.http_options.base_url = Some(base_url.into());
        self
    }

    /// 设置 API 版本。
    #[must_use]
    pub fn api_version(mut self, api_version: impl Into<String>) -> Self {
        self.http_options.api_version = Some(api_version.into());
        self
    }

    /// 设置 OAuth scopes。
    #[must_use]
    pub fn auth_scopes(mut self, scopes: Vec<String>) -> Self {
        self.auth_scopes = Some(scopes);
        self
    }

    /// 构建客户端。
    ///
    /// # Errors
    /// 当配置不完整、参数无效或构建 HTTP 客户端失败时返回错误。
    pub fn build(self) -> Result<Client> {
        let Self {
            api_key,
            credentials,
            backend,
            vertex_project,
            vertex_location,
            http_options,
            auth_scopes,
        } = self;

        let backend = Self::resolve_backend(
            backend,
            vertex_project.as_deref(),
            vertex_location.as_deref(),
        );
        Self::validate_vertex_config(
            backend,
            vertex_project.as_deref(),
            vertex_location.as_deref(),
        )?;
        let credentials = Self::resolve_credentials(backend, api_key.as_deref(), credentials)?;
        let headers = Self::build_headers(&http_options, backend, &credentials)?;
        let http = Self::build_http_client(&http_options, headers)?;

        let auth_scopes = auth_scopes.unwrap_or_else(|| default_auth_scopes(backend));
        let api_key = match &credentials {
            Credentials::ApiKey(key) => Some(key.clone()),
            _ => None,
        };
        let vertex_config = Self::build_vertex_config(backend, vertex_project, vertex_location)?;
        let config = ClientConfig {
            api_key,
            backend,
            vertex_config,
            http_options,
            credentials: credentials.clone(),
            auth_scopes,
        };

        let auth_provider = build_auth_provider(&credentials)?;
        let api_client = ApiClient::new(&config);

        Ok(Client {
            inner: Arc::new(ClientInner {
                http,
                config,
                api_client,
                auth_provider,
            }),
        })
    }

    fn resolve_backend(
        backend: Option<Backend>,
        vertex_project: Option<&str>,
        vertex_location: Option<&str>,
    ) -> Backend {
        backend.unwrap_or_else(|| {
            if vertex_project.is_some() || vertex_location.is_some() {
                Backend::VertexAi
            } else {
                Backend::GeminiApi
            }
        })
    }

    fn validate_vertex_config(
        backend: Backend,
        vertex_project: Option<&str>,
        vertex_location: Option<&str>,
    ) -> Result<()> {
        if backend == Backend::VertexAi && (vertex_project.is_none() || vertex_location.is_none()) {
            return Err(Error::InvalidConfig {
                message: "Project and location required for Vertex AI".into(),
            });
        }
        Ok(())
    }

    fn resolve_credentials(
        backend: Backend,
        api_key: Option<&str>,
        credentials: Option<Credentials>,
    ) -> Result<Credentials> {
        if credentials.is_some()
            && api_key.is_some()
            && !matches!(credentials, Some(Credentials::ApiKey(_)))
        {
            return Err(Error::InvalidConfig {
                message: "API key cannot be combined with OAuth/ADC credentials".into(),
            });
        }

        let credentials = match credentials {
            Some(credentials) => credentials,
            None => {
                if let Some(api_key) = api_key {
                    Credentials::ApiKey(api_key.to_string())
                } else if backend == Backend::VertexAi {
                    Credentials::ApplicationDefault
                } else {
                    return Err(Error::InvalidConfig {
                        message: "API key or OAuth credentials required for Gemini API".into(),
                    });
                }
            }
        };

        if backend == Backend::VertexAi && matches!(credentials, Credentials::ApiKey(_)) {
            return Err(Error::InvalidConfig {
                message: "Vertex AI does not support API key authentication".into(),
            });
        }

        Ok(credentials)
    }

    fn build_headers(
        http_options: &HttpOptions,
        backend: Backend,
        credentials: &Credentials,
    ) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        for (key, value) in &http_options.headers {
            let name =
                HeaderName::from_bytes(key.as_bytes()).map_err(|_| Error::InvalidConfig {
                    message: format!("Invalid header name: {key}"),
                })?;
            let value = HeaderValue::from_str(value).map_err(|_| Error::InvalidConfig {
                message: format!("Invalid header value for {key}"),
            })?;
            headers.insert(name, value);
        }

        if backend == Backend::GeminiApi {
            let api_key = match credentials {
                Credentials::ApiKey(key) => key.as_str(),
                _ => "",
            };
            let header_name = HeaderName::from_static("x-goog-api-key");
            if !api_key.is_empty() && !headers.contains_key(&header_name) {
                let mut header_value =
                    HeaderValue::from_str(api_key).map_err(|_| Error::InvalidConfig {
                        message: "Invalid API key value".into(),
                    })?;
                header_value.set_sensitive(true);
                headers.insert(header_name, header_value);
            }
        }

        Ok(headers)
    }

    fn build_http_client(http_options: &HttpOptions, headers: HeaderMap) -> Result<HttpClient> {
        let mut http_builder = HttpClient::builder();
        if let Some(timeout) = http_options.timeout {
            http_builder = http_builder.timeout(Duration::from_secs(timeout));
        }

        if let Some(proxy_url) = &http_options.proxy {
            let proxy = Proxy::all(proxy_url).map_err(|e| Error::InvalidConfig {
                message: format!("Invalid proxy: {e}"),
            })?;
            http_builder = http_builder.proxy(proxy);
        }

        if !headers.is_empty() {
            http_builder = http_builder.default_headers(headers);
        }

        Ok(http_builder.build()?)
    }

    fn build_vertex_config(
        backend: Backend,
        vertex_project: Option<String>,
        vertex_location: Option<String>,
    ) -> Result<Option<VertexConfig>> {
        if backend != Backend::VertexAi {
            return Ok(None);
        }
        let project = vertex_project.ok_or_else(|| Error::InvalidConfig {
            message: "Project and location required for Vertex AI".into(),
        })?;
        let location = vertex_location.ok_or_else(|| Error::InvalidConfig {
            message: "Project and location required for Vertex AI".into(),
        })?;
        Ok(Some(VertexConfig {
            project,
            location,
            credentials: None,
        }))
    }
}

fn build_auth_provider(credentials: &Credentials) -> Result<Option<AuthProvider>> {
    match credentials {
        Credentials::ApiKey(_) => Ok(None),
        Credentials::OAuth {
            client_secret_path,
            token_cache_path,
        } => Ok(Some(AuthProvider::OAuth(Arc::new(
            OAuthTokenProvider::from_paths(client_secret_path.clone(), token_cache_path.clone())?,
        )))),
        Credentials::ApplicationDefault => Ok(Some(AuthProvider::ApplicationDefault(Arc::new(
            OnceCell::new(),
        )))),
    }
}

#[derive(Clone)]
pub(crate) enum AuthProvider {
    OAuth(Arc<OAuthTokenProvider>),
    ApplicationDefault(Arc<OnceCell<Arc<GoogleCredentials>>>),
}

impl AuthProvider {
    async fn headers(&self, scopes: &[&str]) -> Result<HeaderMap> {
        match self {
            Self::OAuth(provider) => {
                let token = provider.token().await?;
                let mut header =
                    HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| Error::Auth {
                        message: "Invalid OAuth access token".into(),
                    })?;
                header.set_sensitive(true);
                let mut headers = HeaderMap::new();
                headers.insert(AUTHORIZATION, header);
                Ok(headers)
            }
            Self::ApplicationDefault(cell) => {
                let credentials = cell
                    .get_or_try_init(|| async {
                        AuthBuilder::default()
                            .with_scopes(scopes.iter().copied())
                            .build()
                            .map(Arc::new)
                            .map_err(|err| Error::Auth {
                                message: format!("ADC init failed: {err}"),
                            })
                    })
                    .await?;
                let headers = credentials
                    .headers(Extensions::new())
                    .await
                    .map_err(|err| Error::Auth {
                        message: format!("ADC header fetch failed: {err}"),
                    })?;
                match headers {
                    CacheableResource::New { data, .. } => Ok(data),
                    CacheableResource::NotModified => Err(Error::Auth {
                        message: "ADC header fetch returned NotModified without cached headers"
                            .into(),
                    }),
                }
            }
        }
    }
}

impl ClientInner {
    /// 发送请求并自动注入鉴权头。
    ///
    /// # Errors
    /// 当请求构建、鉴权头获取或网络请求失败时返回错误。
    pub async fn send(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let mut request = request.build()?;
        if let Some(headers) = self.auth_headers().await? {
            for (name, value) in &headers {
                if request.headers().contains_key(name) {
                    continue;
                }
                let mut value = value.clone();
                if name == AUTHORIZATION {
                    value.set_sensitive(true);
                }
                request.headers_mut().insert(name.clone(), value);
            }
        }
        #[cfg(feature = "mcp")]
        crate::mcp::append_mcp_usage_header(request.headers_mut())?;
        Ok(self.http.execute(request).await?)
    }

    async fn auth_headers(&self) -> Result<Option<HeaderMap>> {
        let Some(provider) = &self.auth_provider else {
            return Ok(None);
        };

        let scopes: Vec<&str> = self.config.auth_scopes.iter().map(String::as_str).collect();
        let headers = provider.headers(&scopes).await?;
        Ok(Some(headers))
    }
}

fn default_auth_scopes(backend: Backend) -> Vec<String> {
    match backend {
        Backend::VertexAi => vec!["https://www.googleapis.com/auth/cloud-platform".into()],
        Backend::GeminiApi => vec![
            "https://www.googleapis.com/auth/generative-language".into(),
            "https://www.googleapis.com/auth/generative-language.retriever".into(),
        ],
    }
}

pub(crate) struct ApiClient {
    pub base_url: String,
    pub api_version: String,
}

impl ApiClient {
    /// 创建 API 客户端配置。
    pub fn new(config: &ClientConfig) -> Self {
        let base_url = config.http_options.base_url.as_deref().map_or_else(
            || match config.backend {
                Backend::VertexAi => {
                    let location = config
                        .vertex_config
                        .as_ref()
                        .map_or("", |cfg| cfg.location.as_str());
                    if location.is_empty() {
                        "https://aiplatform.googleapis.com/".to_string()
                    } else {
                        format!("https://{location}-aiplatform.googleapis.com/")
                    }
                }
                Backend::GeminiApi => "https://generativelanguage.googleapis.com/".to_string(),
            },
            normalize_base_url,
        );

        let api_version =
            config
                .http_options
                .api_version
                .clone()
                .unwrap_or_else(|| match config.backend {
                    Backend::VertexAi => "v1beta1".to_string(),
                    Backend::GeminiApi => "v1beta".to_string(),
                });

        Self {
            base_url,
            api_version,
        }
    }
}

fn normalize_base_url(base_url: &str) -> String {
    let mut value = base_url.trim().to_string();
    if !value.ends_with('/') {
        value.push('/');
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::with_env;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_client_from_api_key() {
        let client = Client::new("test-api-key").unwrap();
        assert_eq!(client.inner.config.backend, Backend::GeminiApi);
    }

    #[test]
    fn test_client_builder() {
        let client = Client::builder()
            .api_key("test-key")
            .timeout(30)
            .build()
            .unwrap();
        assert!(client.inner.config.api_key.is_some());
    }

    #[test]
    fn test_vertex_ai_config() {
        let client = Client::new_vertex("my-project", "us-central1").unwrap();
        assert_eq!(client.inner.config.backend, Backend::VertexAi);
        assert_eq!(
            client.inner.api_client.base_url,
            "https://us-central1-aiplatform.googleapis.com/"
        );
    }

    #[test]
    fn test_base_url_normalization() {
        let client = Client::builder()
            .api_key("test-key")
            .base_url("https://example.com")
            .build()
            .unwrap();
        assert_eq!(client.inner.api_client.base_url, "https://example.com/");
    }

    #[test]
    fn test_from_env_reads_overrides() {
        with_env(
            &[
                ("GEMINI_API_KEY", Some("env-key")),
                ("GENAI_BASE_URL", Some("https://env.example.com")),
                ("GENAI_API_VERSION", Some("v99")),
                ("GOOGLE_API_KEY", None),
            ],
            || {
                let client = Client::from_env().unwrap();
                assert_eq!(client.inner.api_client.base_url, "https://env.example.com/");
                assert_eq!(client.inner.api_client.api_version, "v99");
            },
        );
    }

    #[test]
    fn test_from_env_ignores_empty_overrides() {
        with_env(
            &[
                ("GEMINI_API_KEY", Some("env-key")),
                ("GENAI_BASE_URL", Some("   ")),
                ("GENAI_API_VERSION", Some("")),
                ("GOOGLE_API_KEY", None),
            ],
            || {
                let client = Client::from_env().unwrap();
                assert_eq!(
                    client.inner.api_client.base_url,
                    "https://generativelanguage.googleapis.com/"
                );
                assert_eq!(client.inner.api_client.api_version, "v1beta");
            },
        );
    }

    #[test]
    fn test_from_env_missing_key_errors() {
        with_env(
            &[
                ("GEMINI_API_KEY", None),
                ("GOOGLE_API_KEY", None),
                ("GENAI_BASE_URL", None),
            ],
            || {
                let result = Client::from_env();
                assert!(result.is_err());
            },
        );
    }

    #[test]
    fn test_from_env_google_api_key_fallback() {
        with_env(
            &[
                ("GEMINI_API_KEY", None),
                ("GOOGLE_API_KEY", Some("google-key")),
            ],
            || {
                let client = Client::from_env().unwrap();
                assert_eq!(client.inner.config.api_key.as_deref(), Some("google-key"));
            },
        );
    }

    #[test]
    fn test_with_oauth_missing_client_secret_errors() {
        let dir = tempdir().unwrap();
        let secret_path = dir.path().join("missing_client_secret.json");
        let err = Client::with_oauth(&secret_path).err().unwrap();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_with_adc_builds_client() {
        let client = Client::with_adc().unwrap();
        assert!(matches!(
            client.inner.config.credentials,
            Credentials::ApplicationDefault
        ));
    }

    #[test]
    fn test_builder_defaults_to_vertex_when_project_set() {
        let client = Client::builder()
            .vertex_project("proj")
            .vertex_location("loc")
            .build()
            .unwrap();
        assert_eq!(client.inner.config.backend, Backend::VertexAi);
        assert!(matches!(
            client.inner.config.credentials,
            Credentials::ApplicationDefault
        ));
    }

    #[test]
    fn test_valid_proxy_is_accepted() {
        let client = Client::builder()
            .api_key("test-key")
            .proxy("http://127.0.0.1:8888")
            .build();
        assert!(client.is_ok());
    }

    #[test]
    fn test_vertex_requires_project_and_location() {
        let result = Client::builder().backend(Backend::VertexAi).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_api_key_with_oauth_is_invalid() {
        let result = Client::builder()
            .api_key("test-key")
            .credentials(Credentials::OAuth {
                client_secret_path: PathBuf::from("client_secret.json"),
                token_cache_path: None,
            })
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_api_key_for_gemini_errors() {
        let result = Client::builder().backend(Backend::GeminiApi).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_header_name_is_rejected() {
        let result = Client::builder()
            .api_key("test-key")
            .header("bad header", "value")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_header_value_is_rejected() {
        let result = Client::builder()
            .api_key("test-key")
            .header("x-test", "bad\nvalue")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_api_key_value_is_rejected() {
        let err = Client::builder().api_key("bad\nkey").build().err().unwrap();
        assert!(
            matches!(err, Error::InvalidConfig { message } if message.contains("Invalid API key value"))
        );
    }

    #[test]
    fn test_invalid_proxy_is_rejected() {
        let result = Client::builder()
            .api_key("test-key")
            .proxy("not a url")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_vertex_api_key_is_rejected() {
        let result = Client::builder()
            .backend(Backend::VertexAi)
            .vertex_project("proj")
            .vertex_location("loc")
            .credentials(Credentials::ApiKey("key".into()))
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_default_auth_scopes() {
        let gemini = default_auth_scopes(Backend::GeminiApi);
        assert!(gemini.iter().any(|s| s.contains("generative-language")));

        let vertex = default_auth_scopes(Backend::VertexAi);
        assert!(vertex.iter().any(|s| s.contains("cloud-platform")));
    }

    #[test]
    fn test_custom_auth_scopes_override_default() {
        let client = Client::builder()
            .api_key("test-key")
            .auth_scopes(vec!["scope-1".to_string()])
            .build()
            .unwrap();
        assert_eq!(client.inner.config.auth_scopes, vec!["scope-1".to_string()]);
    }
}
