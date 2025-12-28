//! Interactions API surface.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures_util::Stream;
use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::interactions::{
    CancelInteractionConfig, CreateInteractionConfig, DeleteInteractionConfig,
    GetInteractionConfig, Interaction, InteractionEvent,
};
use serde_json::Value;

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};
use crate::sse::parse_sse_stream_with;

#[derive(Clone)]
pub struct Interactions {
    pub(crate) inner: Arc<ClientInner>,
}

impl Interactions {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建 Interaction。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn create(&self, config: CreateInteractionConfig) -> Result<Interaction> {
        self.create_with_config(config).await
    }

    /// 创建 Interaction（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn create_with_config(
        &self,
        mut config: CreateInteractionConfig,
    ) -> Result<Interaction> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let url = build_interactions_url(&self.inner, http_options.as_ref());
        let mut request = self.inner.http.post(url).json(&config);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        parse_interaction_response(response).await
    }

    /// 创建 Interaction（流式 SSE）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn create_stream(
        &self,
        mut config: CreateInteractionConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<InteractionEvent>> + Send>>> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let mut url = build_interactions_url(&self.inner, http_options.as_ref());
        url.push_str("?alt=sse");
        let mut request = self.inner.http.post(url).json(&config);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let stream = parse_sse_stream_with::<InteractionEvent>(response);
        Ok(Box::pin(stream))
    }

    /// 获取 Interaction。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn get(&self, id: impl AsRef<str>) -> Result<Interaction> {
        self.get_with_config(id, GetInteractionConfig::default())
            .await
    }

    /// 获取 Interaction（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn get_with_config(
        &self,
        id: impl AsRef<str>,
        mut config: GetInteractionConfig,
    ) -> Result<Interaction> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_interaction_name(id.as_ref());
        let url = build_interaction_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        parse_interaction_response(response).await
    }

    /// 删除 Interaction。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn delete(&self, id: impl AsRef<str>) -> Result<()> {
        self.delete_with_config(id, DeleteInteractionConfig::default())
            .await
    }

    /// 删除 Interaction（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn delete_with_config(
        &self,
        id: impl AsRef<str>,
        mut config: DeleteInteractionConfig,
    ) -> Result<()> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_interaction_name(id.as_ref());
        let url = build_interaction_url(&self.inner, &name, http_options.as_ref());
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

    /// 取消 Interaction。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn cancel(&self, id: impl AsRef<str>) -> Result<Interaction> {
        self.cancel_with_config(id, CancelInteractionConfig::default())
            .await
    }

    /// 取消 Interaction（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn cancel_with_config(
        &self,
        id: impl AsRef<str>,
        mut config: CancelInteractionConfig,
    ) -> Result<Interaction> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_interaction_name(id.as_ref());
        let url = build_interaction_cancel_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.post(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        parse_interaction_response(response).await
    }
}

fn ensure_gemini_backend(inner: &ClientInner) -> Result<()> {
    if inner.config.backend != Backend::GeminiApi {
        return Err(Error::InvalidConfig {
            message: "Interactions API is only supported for Gemini API backend".into(),
        });
    }
    Ok(())
}

fn normalize_interaction_name(name: &str) -> String {
    if name.starts_with("interactions/") {
        name.to_string()
    } else {
        format!("interactions/{name}")
    }
}

fn build_interactions_url(
    inner: &ClientInner,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/interactions")
}

fn build_interaction_url(
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

fn build_interaction_cancel_url(
    inner: &ClientInner,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    format!(
        "{}:cancel",
        build_interaction_url(inner, name, http_options)
    )
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

async fn parse_interaction_response(response: reqwest::Response) -> Result<Interaction> {
    let text = response.text().await.unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(Interaction::default());
    }
    let value: Value = serde_json::from_str(&text)?;
    let interaction: Interaction = serde_json::from_value(value)?;
    Ok(interaction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_client_inner;

    #[test]
    fn test_normalize_names_and_urls() {
        assert_eq!(
            normalize_interaction_name("interactions/1"),
            "interactions/1"
        );
        assert_eq!(normalize_interaction_name("1"), "interactions/1");

        let gemini = test_client_inner(Backend::GeminiApi);
        let url = build_interactions_url(&gemini, None);
        assert!(url.ends_with("/v1beta/interactions"));
        let url = build_interaction_url(&gemini, "interactions/1", None);
        assert!(url.ends_with("/v1beta/interactions/1"));
        let url = build_interaction_cancel_url(&gemini, "interactions/1", None);
        assert!(url.ends_with("/v1beta/interactions/1:cancel"));
    }

    #[test]
    fn test_backend_check_and_invalid_header() {
        let vertex = test_client_inner(Backend::VertexAi);
        let err = ensure_gemini_backend(&vertex).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

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
}
