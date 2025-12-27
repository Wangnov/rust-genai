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
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::builder()
            .api_key(api_key)
            .backend(Backend::GeminiApi)
            .build()
    }

    /// 从环境变量创建客户端。
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
    pub fn new_vertex(project: impl Into<String>, location: impl Into<String>) -> Result<Self> {
        Self::builder()
            .backend(Backend::VertexAi)
            .vertex_project(project)
            .vertex_location(location)
            .build()
    }

    /// 使用 OAuth 凭据创建客户端（默认读取 token.json）。
    pub fn with_oauth(client_secret_path: impl AsRef<Path>) -> Result<Self> {
        Self::builder()
            .credentials(Credentials::OAuth {
                client_secret_path: client_secret_path.as_ref().to_path_buf(),
                token_cache_path: None,
            })
            .build()
    }

    /// 使用 Application Default Credentials 创建客户端。
    pub fn with_adc() -> Result<Self> {
        Self::builder()
            .credentials(Credentials::ApplicationDefault)
            .build()
    }

    /// 创建 Builder。
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// 访问 Models API。
    pub fn models(&self) -> crate::models::Models {
        crate::models::Models::new(self.inner.clone())
    }

    /// 访问 Chats API。
    pub fn chats(&self) -> crate::chats::Chats {
        crate::chats::Chats::new(self.inner.clone())
    }

    /// 访问 Files API。
    pub fn files(&self) -> crate::files::Files {
        crate::files::Files::new(self.inner.clone())
    }

    /// 访问 FileSearchStores API。
    pub fn file_search_stores(&self) -> crate::file_search_stores::FileSearchStores {
        crate::file_search_stores::FileSearchStores::new(self.inner.clone())
    }

    /// 访问 Documents API。
    pub fn documents(&self) -> crate::documents::Documents {
        crate::documents::Documents::new(self.inner.clone())
    }

    /// 访问 Live API。
    pub fn live(&self) -> crate::live::Live {
        crate::live::Live::new(self.inner.clone())
    }

    /// 访问 Live Music API。
    pub fn live_music(&self) -> crate::live_music::LiveMusic {
        crate::live_music::LiveMusic::new(self.inner.clone())
    }

    /// 访问 Caches API。
    pub fn caches(&self) -> crate::caches::Caches {
        crate::caches::Caches::new(self.inner.clone())
    }

    /// 访问 Batches API。
    pub fn batches(&self) -> crate::batches::Batches {
        crate::batches::Batches::new(self.inner.clone())
    }

    /// 访问 Tunings API。
    pub fn tunings(&self) -> crate::tunings::Tunings {
        crate::tunings::Tunings::new(self.inner.clone())
    }

    /// 访问 Operations API。
    pub fn operations(&self) -> crate::operations::Operations {
        crate::operations::Operations::new(self.inner.clone())
    }

    /// 访问 AuthTokens API（Ephemeral Tokens）。
    pub fn auth_tokens(&self) -> crate::tokens::AuthTokens {
        crate::tokens::AuthTokens::new(self.inner.clone())
    }

    /// 访问 Interactions API。
    pub fn interactions(&self) -> crate::interactions::Interactions {
        crate::interactions::Interactions::new(self.inner.clone())
    }

    /// 访问 Deep Research。
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
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// 设置认证方式（OAuth/ADC/API Key）。
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// 设置后端（Gemini API 或 Vertex AI）。
    pub fn backend(mut self, backend: Backend) -> Self {
        self.backend = Some(backend);
        self
    }

    /// 设置 Vertex AI 项目 ID。
    pub fn vertex_project(mut self, project: impl Into<String>) -> Self {
        self.vertex_project = Some(project.into());
        self
    }

    /// 设置 Vertex AI 区域。
    pub fn vertex_location(mut self, location: impl Into<String>) -> Self {
        self.vertex_location = Some(location.into());
        self
    }

    /// 设置请求超时（秒）。
    pub fn timeout(mut self, secs: u64) -> Self {
        self.http_options.timeout = Some(secs);
        self
    }

    /// 设置代理。
    pub fn proxy(mut self, url: impl Into<String>) -> Self {
        self.http_options.proxy = Some(url.into());
        self
    }

    /// 增加默认 HTTP 头。
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.http_options.headers.insert(key.into(), value.into());
        self
    }

    /// 设置自定义基础 URL。
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.http_options.base_url = Some(base_url.into());
        self
    }

    /// 设置 API 版本。
    pub fn api_version(mut self, api_version: impl Into<String>) -> Self {
        self.http_options.api_version = Some(api_version.into());
        self
    }

    /// 设置 OAuth scopes。
    pub fn auth_scopes(mut self, scopes: Vec<String>) -> Self {
        self.auth_scopes = Some(scopes);
        self
    }

    /// 构建客户端。
    pub fn build(self) -> Result<Client> {
        let backend = self.backend.unwrap_or_else(|| {
            if self.vertex_project.is_some() || self.vertex_location.is_some() {
                Backend::VertexAi
            } else {
                Backend::GeminiApi
            }
        });

        if backend == Backend::VertexAi
            && (self.vertex_project.is_none() || self.vertex_location.is_none())
        {
            return Err(Error::InvalidConfig {
                message: "Project and location required for Vertex AI".into(),
            });
        }

        if self.credentials.is_some()
            && self.api_key.is_some()
            && !matches!(self.credentials, Some(Credentials::ApiKey(_)))
        {
            return Err(Error::InvalidConfig {
                message: "API key cannot be combined with OAuth/ADC credentials".into(),
            });
        }

        let credentials = match self.credentials {
            Some(credentials) => credentials,
            None => {
                if let Some(api_key) = self.api_key.clone() {
                    Credentials::ApiKey(api_key)
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

        let mut headers = HeaderMap::new();
        for (key, value) in &self.http_options.headers {
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
            let api_key = match &credentials {
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

        let mut http_builder = HttpClient::builder();
        if let Some(timeout) = self.http_options.timeout {
            http_builder = http_builder.timeout(Duration::from_secs(timeout));
        }

        if let Some(proxy_url) = &self.http_options.proxy {
            let proxy = Proxy::all(proxy_url).map_err(|e| Error::InvalidConfig {
                message: format!("Invalid proxy: {e}"),
            })?;
            http_builder = http_builder.proxy(proxy);
        }

        if !headers.is_empty() {
            http_builder = http_builder.default_headers(headers);
        }

        let http = http_builder.build()?;

        let auth_scopes = self
            .auth_scopes
            .unwrap_or_else(|| default_auth_scopes(backend));
        let api_key = match &credentials {
            Credentials::ApiKey(key) => Some(key.clone()),
            _ => None,
        };
        let config = ClientConfig {
            api_key,
            backend,
            vertex_config: if backend == Backend::VertexAi {
                Some(VertexConfig {
                    project: self.vertex_project.unwrap(),
                    location: self.vertex_location.unwrap(),
                    credentials: None,
                })
            } else {
                None
            },
            http_options: self.http_options,
            credentials: credentials.clone(),
            auth_scopes,
        };

        let auth_provider = match &credentials {
            Credentials::ApiKey(_) => None,
            Credentials::OAuth {
                client_secret_path,
                token_cache_path,
            } => Some(AuthProvider::OAuth(Arc::new(
                OAuthTokenProvider::from_paths(
                    client_secret_path.clone(),
                    token_cache_path.clone(),
                )?,
            ))),
            Credentials::ApplicationDefault => {
                Some(AuthProvider::ApplicationDefault(Arc::new(OnceCell::new())))
            }
        };

        let api_client = ApiClient::new(&config)?;

        Ok(Client {
            inner: Arc::new(ClientInner {
                http,
                config,
                api_client,
                auth_provider,
            }),
        })
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
            AuthProvider::OAuth(provider) => {
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
            AuthProvider::ApplicationDefault(cell) => {
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
    pub async fn send(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let mut request = request.build()?;
        if let Some(headers) = self.auth_headers().await? {
            for (name, value) in headers.iter() {
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
        maybe_add_mcp_usage_header(request.headers_mut())?;
        Ok(self.http.execute(request).await?)
    }

    async fn auth_headers(&self) -> Result<Option<HeaderMap>> {
        let provider = match &self.auth_provider {
            Some(provider) => provider,
            None => return Ok(None),
        };

        let scopes: Vec<&str> = self.config.auth_scopes.iter().map(|s| s.as_str()).collect();
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
    pub fn new(config: &ClientConfig) -> Result<Self> {
        let base_url = if let Some(base_url) = &config.http_options.base_url {
            normalize_base_url(base_url)
        } else {
            match config.backend {
                Backend::VertexAi => {
                    let location = config
                        .vertex_config
                        .as_ref()
                        .map(|cfg| cfg.location.as_str())
                        .unwrap_or("");
                    if location.is_empty() {
                        "https://aiplatform.googleapis.com/".to_string()
                    } else {
                        format!("https://{location}-aiplatform.googleapis.com/")
                    }
                }
                Backend::GeminiApi => "https://generativelanguage.googleapis.com/".to_string(),
            }
        };

        let api_version =
            config
                .http_options
                .api_version
                .clone()
                .unwrap_or_else(|| match config.backend {
                    Backend::VertexAi => "v1beta1".to_string(),
                    Backend::GeminiApi => "v1beta".to_string(),
                });

        Ok(Self {
            base_url,
            api_version,
        })
    }
}

fn normalize_base_url(base_url: &str) -> String {
    let mut value = base_url.trim().to_string();
    if !value.ends_with('/') {
        value.push('/');
    }
    value
}

#[cfg(feature = "mcp")]
fn maybe_add_mcp_usage_header(headers: &mut HeaderMap) -> Result<()> {
    crate::mcp::append_mcp_usage_header(headers)
}

#[cfg(not(feature = "mcp"))]
fn maybe_add_mcp_usage_header(_headers: &mut HeaderMap) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
