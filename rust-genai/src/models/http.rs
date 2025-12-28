use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};
use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::models::ListModelsConfig;
use serde_json::Value;
use std::time::Duration;

pub(super) fn transform_model_name(backend: Backend, model: &str) -> String {
    match backend {
        Backend::GeminiApi => {
            if model.starts_with("models/") {
                model.to_string()
            } else {
                format!("models/{model}")
            }
        }
        Backend::VertexAi => {
            if model.starts_with("projects/") || model.starts_with("publishers/") {
                model.to_string()
            } else {
                format!("publishers/google/models/{model}")
            }
        }
    }
}

pub(super) fn build_model_method_url(
    inner: &ClientInner,
    model: &str,
    method: &str,
) -> Result<String> {
    let model = transform_model_name(inner.config.backend, model);
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/{model}:{method}"),
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
                "{base}{version}/projects/{}/locations/{}/{}:{method}",
                vertex.project, vertex.location, model
            )
        }
    };
    Ok(url)
}

pub(super) fn build_model_get_url(inner: &ClientInner, model: &str) -> Result<String> {
    let model = transform_model_name(inner.config.backend, model);
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/{model}"),
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
                "{base}{version}/projects/{}/locations/{}/{}",
                vertex.project, vertex.location, model
            )
        }
    };
    Ok(url)
}

pub(super) fn build_model_get_url_with_options(
    inner: &ClientInner,
    model: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let model = transform_model_name(inner.config.backend, model);
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/{model}"),
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
                "{base}{version}/projects/{}/locations/{}/{}",
                vertex.project, vertex.location, model
            )
        }
    };
    Ok(url)
}

pub(super) fn build_models_list_url(
    inner: &ClientInner,
    config: &ListModelsConfig,
) -> Result<String> {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/models"),
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
                "{base}{version}/projects/{}/locations/{}/publishers/google/models",
                vertex.project, vertex.location
            )
        }
    };
    add_list_query_params(&url, config)
}

pub(super) fn add_list_query_params(url: &str, config: &ListModelsConfig) -> Result<String> {
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
        if let Some(query_base) = config.query_base {
            pairs.append_pair("queryBase", if query_base { "true" } else { "false" });
        }
    }
    Ok(url.to_string())
}

pub(super) fn apply_http_options(
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

pub(super) fn merge_extra_body(
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
    use crate::client::{Backend, Client};
    use crate::error::Error;
    use crate::test_support::test_vertex_inner_missing_config;
    use rust_genai_types::http::HttpOptions as TypesHttpOptions;
    use rust_genai_types::models::ListModelsConfig;
    use serde_json::json;

    #[test]
    fn test_transform_model_name() {
        assert_eq!(
            transform_model_name(Backend::GeminiApi, "gemini-1.5-pro"),
            "models/gemini-1.5-pro"
        );
        assert_eq!(
            transform_model_name(Backend::VertexAi, "gemini-1.5-pro"),
            "publishers/google/models/gemini-1.5-pro"
        );
    }

    #[test]
    fn test_build_model_urls() {
        let client = Client::new("test-key").unwrap();
        let models = client.models();
        let url =
            build_model_method_url(&models.inner, "gemini-1.5-pro", "generateContent").unwrap();
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent"
        );
    }

    #[test]
    fn test_model_urls_and_list_params_with_options() {
        let client = Client::new("test-key").unwrap();
        let models = client.models();
        let opts = rust_genai_types::http::HttpOptions {
            base_url: Some("https://example.com/".to_string()),
            api_version: Some("v1".to_string()),
            ..Default::default()
        };
        let url =
            build_model_get_url_with_options(&models.inner, "gemini-1.5-pro", Some(&opts)).unwrap();
        assert_eq!(url, "https://example.com/v1/models/gemini-1.5-pro");

        let url = build_models_list_url(
            &models.inner,
            &ListModelsConfig {
                page_size: Some(3),
                page_token: Some("t".to_string()),
                filter: Some("state=ACTIVE".to_string()),
                query_base: Some(true),
            },
        )
        .unwrap();
        assert!(url.contains("pageSize=3"));
        assert!(url.contains("pageToken=t"));
        assert!(url.contains("queryBase=true"));
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
    fn test_vertex_missing_config_errors() {
        let inner = test_vertex_inner_missing_config();
        let err = build_model_method_url(&inner, "models/1", "generateContent").unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
        let err = build_model_get_url(&inner, "models/1").unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
        let err = build_models_list_url(&inner, &ListModelsConfig::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_models_misc_branches() {
        let err = add_list_query_params("http://[::1", &ListModelsConfig::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let inner = test_vertex_inner_missing_config();
        let err = build_model_get_url_with_options(&inner, "gemini-1.5-pro", None).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = TypesHttpOptions {
            headers: Some([("x-test".to_string(), "bad\nvalue".to_string())].into()),
            ..Default::default()
        };
        let err = apply_http_options(request, Some(&options)).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_merge_extra_body() {
        let mut body = json!({"a": 1});
        let mut options = rust_genai_types::http::HttpOptions {
            extra_body: Some(json!({"b": 2})),
            ..Default::default()
        };
        merge_extra_body(&mut body, &options).unwrap();
        assert_eq!(body.get("b").and_then(serde_json::Value::as_i64), Some(2));

        let mut bad_body = json!(["not object"]);
        options.extra_body = Some(json!("bad"));
        let err = merge_extra_body(&mut bad_body, &options).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }
}
