//! Webhooks API surface.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::webhooks::{
    CreateWebhookConfig, DeleteWebhookConfig, GetWebhookConfig, ListWebhooksConfig,
    PingWebhookConfig, RotateWebhookSigningSecretConfig, UpdateWebhookConfig, Webhook,
    WebhookDeleteResponse, WebhookListResponse, WebhookPingResponse,
    WebhookRotateSigningSecretResponse,
};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct Webhooks {
    pub(crate) inner: Arc<ClientInner>,
}

impl Webhooks {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// Creates a webhook.
    ///
    /// # Errors
    /// Returns an error when the request fails or the config is invalid.
    pub async fn create(&self, mut config: CreateWebhookConfig) -> Result<Webhook> {
        ensure_gemini_backend(&self.inner)?;
        validate_create_config(&config)?;
        let http_options = config.http_options.take();
        let url = add_create_query_params(
            &build_webhooks_url(&self.inner, http_options.as_ref()),
            config.webhook_id.as_deref(),
        )?;
        let mut request = self.inner.http.post(url).json(&config);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        parse_json_or_default::<Webhook>(response).await
    }

    /// Updates a webhook.
    ///
    /// # Errors
    /// Returns an error when the request fails or the config is invalid.
    pub async fn update(
        &self,
        id: impl AsRef<str>,
        mut config: UpdateWebhookConfig,
    ) -> Result<Webhook> {
        ensure_gemini_backend(&self.inner)?;
        validate_update_config(&config)?;
        let http_options = config.http_options.take();
        let name = normalize_webhook_name(id.as_ref());
        let url = add_update_query_params(
            &build_webhook_url(&self.inner, &name, http_options.as_ref()),
            config.update_mask.as_deref(),
        )?;
        let mut request = self.inner.http.patch(url).json(&config);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        parse_json_or_default::<Webhook>(response).await
    }

    /// Lists webhooks.
    ///
    /// # Errors
    /// Returns an error when the request fails or the config is invalid.
    pub async fn list(&self) -> Result<WebhookListResponse> {
        self.list_with_config(ListWebhooksConfig::default()).await
    }

    /// Lists webhooks with pagination config.
    ///
    /// # Errors
    /// Returns an error when the request fails or the config is invalid.
    pub async fn list_with_config(
        &self,
        mut config: ListWebhooksConfig,
    ) -> Result<WebhookListResponse> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let url = add_list_query_params(
            &build_webhooks_url(&self.inner, http_options.as_ref()),
            &config,
        )?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        parse_json_or_default::<WebhookListResponse>(response).await
    }

    /// Deletes a webhook.
    ///
    /// # Errors
    /// Returns an error when the request fails.
    pub async fn delete(&self, id: impl AsRef<str>) -> Result<WebhookDeleteResponse> {
        self.delete_with_config(id, DeleteWebhookConfig::default())
            .await
    }

    /// Deletes a webhook with config.
    ///
    /// # Errors
    /// Returns an error when the request fails.
    pub async fn delete_with_config(
        &self,
        id: impl AsRef<str>,
        mut config: DeleteWebhookConfig,
    ) -> Result<WebhookDeleteResponse> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_webhook_name(id.as_ref());
        let url = build_webhook_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.delete(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        parse_json_or_default::<WebhookDeleteResponse>(response).await
    }

    /// Gets a webhook.
    ///
    /// # Errors
    /// Returns an error when the request fails.
    pub async fn get(&self, id: impl AsRef<str>) -> Result<Webhook> {
        self.get_with_config(id, GetWebhookConfig::default()).await
    }

    /// Gets a webhook with config.
    ///
    /// # Errors
    /// Returns an error when the request fails.
    pub async fn get_with_config(
        &self,
        id: impl AsRef<str>,
        mut config: GetWebhookConfig,
    ) -> Result<Webhook> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_webhook_name(id.as_ref());
        let url = build_webhook_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        parse_json_or_default::<Webhook>(response).await
    }

    /// Sends a ping event to a webhook.
    ///
    /// # Errors
    /// Returns an error when the request fails.
    pub async fn ping(&self, id: impl AsRef<str>) -> Result<WebhookPingResponse> {
        self.ping_with_config(id, PingWebhookConfig::default())
            .await
    }

    /// Sends a ping event to a webhook with config.
    ///
    /// # Errors
    /// Returns an error when the request fails.
    pub async fn ping_with_config(
        &self,
        id: impl AsRef<str>,
        mut config: PingWebhookConfig,
    ) -> Result<WebhookPingResponse> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_webhook_name(id.as_ref());
        let url = build_webhook_ping_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.post(url).json(&serde_json::json!({}));
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        parse_json_or_default::<WebhookPingResponse>(response).await
    }

    /// Rotates the signing secret for a webhook.
    ///
    /// # Errors
    /// Returns an error when the request fails.
    pub async fn rotate_signing_secret(
        &self,
        id: impl AsRef<str>,
    ) -> Result<WebhookRotateSigningSecretResponse> {
        self.rotate_signing_secret_with_config(id, RotateWebhookSigningSecretConfig::default())
            .await
    }

    /// Rotates the signing secret for a webhook with config.
    ///
    /// # Errors
    /// Returns an error when the request fails.
    pub async fn rotate_signing_secret_with_config(
        &self,
        id: impl AsRef<str>,
        mut config: RotateWebhookSigningSecretConfig,
    ) -> Result<WebhookRotateSigningSecretResponse> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_webhook_name(id.as_ref());
        let url = build_webhook_rotate_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.post(url).json(&config);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        parse_json_or_default::<WebhookRotateSigningSecretResponse>(response).await
    }
}

fn ensure_gemini_backend(inner: &ClientInner) -> Result<()> {
    if inner.config.backend != Backend::GeminiApi {
        return Err(Error::InvalidConfig {
            message: "Webhooks API is only supported for Gemini API backend".into(),
        });
    }
    Ok(())
}

fn validate_create_config(config: &CreateWebhookConfig) -> Result<()> {
    if config.uri.trim().is_empty() {
        return Err(Error::InvalidConfig {
            message: "Webhook uri is empty".into(),
        });
    }
    if config.subscribed_events.is_empty() {
        return Err(Error::InvalidConfig {
            message: "Webhook subscribed_events is empty".into(),
        });
    }
    Ok(())
}

fn validate_update_config(config: &UpdateWebhookConfig) -> Result<()> {
    if config.uri.trim().is_empty() {
        return Err(Error::InvalidConfig {
            message: "Webhook uri is empty".into(),
        });
    }
    if config.subscribed_events.is_empty() {
        return Err(Error::InvalidConfig {
            message: "Webhook subscribed_events is empty".into(),
        });
    }
    Ok(())
}

fn normalize_webhook_name(name: &str) -> String {
    if name.starts_with("webhooks/") {
        name.to_string()
    } else {
        format!("webhooks/{name}")
    }
}

fn build_webhooks_url(
    inner: &ClientInner,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/webhooks")
}

fn build_webhook_url(
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

fn build_webhook_ping_url(
    inner: &ClientInner,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    format!("{}:ping", build_webhook_url(inner, name, http_options))
}

fn build_webhook_rotate_url(
    inner: &ClientInner,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    format!(
        "{}:rotateSigningSecret",
        build_webhook_url(inner, name, http_options)
    )
}

fn add_create_query_params(url: &str, webhook_id: Option<&str>) -> Result<String> {
    let mut url = reqwest::Url::parse(url).map_err(|err| Error::InvalidConfig {
        message: err.to_string(),
    })?;
    if let Some(webhook_id) = webhook_id.filter(|value| !value.trim().is_empty()) {
        url.query_pairs_mut().append_pair("webhook_id", webhook_id);
    }
    Ok(url.to_string())
}

fn add_update_query_params(url: &str, update_mask: Option<&str>) -> Result<String> {
    let mut url = reqwest::Url::parse(url).map_err(|err| Error::InvalidConfig {
        message: err.to_string(),
    })?;
    if let Some(update_mask) = update_mask.filter(|value| !value.trim().is_empty()) {
        url.query_pairs_mut()
            .append_pair("update_mask", update_mask);
    }
    Ok(url.to_string())
}

fn add_list_query_params(url: &str, config: &ListWebhooksConfig) -> Result<String> {
    let mut url = reqwest::Url::parse(url).map_err(|err| Error::InvalidConfig {
        message: err.to_string(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(page_size) = config.page_size {
            pairs.append_pair("page_size", &page_size.to_string());
        }
        if let Some(page_token) = &config.page_token {
            if !page_token.is_empty() {
                pairs.append_pair("page_token", page_token);
            }
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

async fn parse_json_or_default<T>(response: reqwest::Response) -> Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    let text = response.text().await.unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(T::default());
    }
    Ok(serde_json::from_str(&text)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use reqwest::header::HeaderMap;

    use crate::client::{ApiClient, ClientConfig};
    use crate::{ClientBuilder, Credentials, HttpOptions, VertexConfig};

    fn test_inner(backend: Backend) -> ClientInner {
        ClientInner {
            http: reqwest::Client::new(),
            config: ClientConfig {
                api_key: Some("test-key".to_string()),
                backend,
                vertex_config: Some(VertexConfig {
                    project: "proj".to_string(),
                    location: "loc".to_string(),
                    credentials: None,
                }),
                http_options: HttpOptions::default(),
                credentials: Credentials::ApiKey("test-key".to_string()),
                auth_scopes: vec![],
            },
            api_client: ApiClient {
                base_url: "https://example.com/".to_string(),
                api_version: "v1beta".to_string(),
            },
            auth_provider: None,
        }
    }

    #[test]
    fn test_normalize_webhook_name() {
        assert_eq!(normalize_webhook_name("hook-1"), "webhooks/hook-1");
        assert_eq!(normalize_webhook_name("webhooks/hook-1"), "webhooks/hook-1");
    }

    #[test]
    fn test_webhook_urls_and_query_params() {
        let inner = test_inner(Backend::GeminiApi);
        let url = build_webhooks_url(&inner, None);
        assert_eq!(url, "https://example.com/v1beta/webhooks");

        let url = build_webhook_url(&inner, "webhooks/hook-1", None);
        assert_eq!(url, "https://example.com/v1beta/webhooks/hook-1");

        let url = build_webhook_ping_url(&inner, "webhooks/hook-1", None);
        assert_eq!(url, "https://example.com/v1beta/webhooks/hook-1:ping");

        let url = build_webhook_rotate_url(&inner, "webhooks/hook-1", None);
        assert_eq!(
            url,
            "https://example.com/v1beta/webhooks/hook-1:rotateSigningSecret"
        );

        let url =
            add_create_query_params(&build_webhooks_url(&inner, None), Some("hook-1")).unwrap();
        assert!(url.contains("webhook_id=hook-1"));

        let url = add_update_query_params(
            &build_webhook_url(&inner, "webhooks/hook-1", None),
            Some("uri,subscribed_events"),
        )
        .unwrap();
        assert!(url.contains("update_mask=uri%2Csubscribed_events"));

        let url = add_list_query_params(
            &build_webhooks_url(&inner, None),
            &ListWebhooksConfig {
                page_size: Some(10),
                page_token: Some("page-2".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(url.contains("page_size=10"));
        assert!(url.contains("page_token=page-2"));
    }

    #[test]
    fn test_webhook_validation() {
        let err = validate_create_config(&CreateWebhookConfig::new("", vec!["x".to_string()]))
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let err =
            validate_update_config(&UpdateWebhookConfig::new("https://example.com", Vec::new()))
                .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_webhooks_require_gemini_backend() {
        let client = ClientBuilder::default()
            .backend(Backend::VertexAi)
            .vertex_project("proj")
            .vertex_location("loc")
            .build()
            .unwrap();
        let webhooks = client.webhooks();
        let err = webhooks.list();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let err = runtime.block_on(err).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
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
    fn test_parse_json_or_default_empty_body() {
        let _headers = HeaderMap::new();
        let inner = Arc::new(test_inner(Backend::GeminiApi));
        let service = Webhooks::new(inner);
        let config =
            CreateWebhookConfig::new("https://example.com", vec!["batch.succeeded".into()]);
        assert_eq!(service.inner.config.backend, Backend::GeminiApi);
        assert_eq!(config.uri, "https://example.com");
    }
}
