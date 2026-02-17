//! Ephemeral auth tokens API.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::config::GenerationConfig;
use rust_genai_types::live_types::{LiveClientSetup, LiveConnectConfig};
use rust_genai_types::tokens::{AuthToken, CreateAuthTokenConfig, LiveConnectConstraints};
use serde_json::{json, Map, Value};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct AuthTokens {
    pub(crate) inner: Arc<ClientInner>,
}

impl AuthTokens {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建 Live API 的 Ephemeral Token。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn create(&self, mut config: CreateAuthTokenConfig) -> Result<AuthToken> {
        ensure_gemini_backend(&self.inner)?;

        let http_options = config.http_options.take();
        let mut body = build_auth_token_body(&config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }

        let url = build_auth_tokens_url(&self.inner, http_options.as_ref());
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
        Ok(response.json::<AuthToken>().await?)
    }
}

fn ensure_gemini_backend(inner: &ClientInner) -> Result<()> {
    if inner.config.backend == Backend::VertexAi {
        return Err(Error::InvalidConfig {
            message: "AuthTokens API is only supported in Gemini API".into(),
        });
    }
    Ok(())
}

fn build_auth_tokens_url(
    inner: &ClientInner,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/auth_tokens")
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

fn build_auth_token_body(config: &CreateAuthTokenConfig) -> Result<Value> {
    let mut body = Map::new();

    if let Some(value) = &config.expire_time {
        body.insert("expireTime".to_string(), json!(value));
    }
    if let Some(value) = &config.new_session_expire_time {
        body.insert("newSessionExpireTime".to_string(), json!(value));
    }
    if let Some(value) = config.uses {
        body.insert("uses".to_string(), json!(value));
    }
    if let Some(constraints) = &config.live_connect_constraints {
        let setup = build_live_setup_for_constraints(constraints);
        let setup_value = serde_json::to_value(setup)?;
        body.insert(
            "bidiGenerateContentSetup".to_string(),
            json!({ "setup": setup_value }),
        );
    }
    if let Some(fields) = &config.lock_additional_fields {
        body.insert("fieldMask".to_string(), serde_json::to_value(fields)?);
    }

    let body = convert_bidi_setup_to_token_setup(body, config.lock_additional_fields.as_ref());
    Ok(Value::Object(body))
}

fn build_live_setup_for_constraints(constraints: &LiveConnectConstraints) -> LiveClientSetup {
    let config = constraints.config.clone().unwrap_or_default();
    let generation_config = merge_generation_config(&config);
    let model = constraints
        .model
        .as_ref()
        .map(|value| normalize_model_name(value));

    LiveClientSetup {
        model,
        generation_config,
        system_instruction: config.system_instruction.clone(),
        tools: config.tools.clone(),
        realtime_input_config: config.realtime_input_config.clone(),
        session_resumption: config.session_resumption.clone(),
        context_window_compression: config.context_window_compression.clone(),
        input_audio_transcription: config.input_audio_transcription.clone(),
        output_audio_transcription: config.output_audio_transcription.clone(),
        proactivity: config.proactivity.clone(),
        explicit_vad_signal: config.explicit_vad_signal,
    }
}

fn merge_generation_config(config: &LiveConnectConfig) -> Option<GenerationConfig> {
    let mut generation_config = config.generation_config.clone().unwrap_or_default();
    let updated = config.generation_config.is_some()
        || config.response_modalities.is_some()
        || config.temperature.is_some()
        || config.top_p.is_some()
        || config.top_k.is_some()
        || config.max_output_tokens.is_some()
        || config.media_resolution.is_some()
        || config.seed.is_some()
        || config.speech_config.is_some()
        || config.thinking_config.is_some()
        || config.enable_affective_dialog.is_some();

    if let Some(value) = config.response_modalities.clone() {
        generation_config.response_modalities = Some(value);
    }
    if let Some(value) = config.temperature {
        generation_config.temperature = Some(value);
    }
    if let Some(value) = config.top_p {
        generation_config.top_p = Some(value);
    }
    if let Some(value) = config.top_k {
        let top_k_value = i16::try_from(value).unwrap_or_else(|_| {
            if value > i32::from(i16::MAX) {
                i16::MAX
            } else {
                i16::MIN
            }
        });
        generation_config.top_k = Some(f32::from(top_k_value));
    }
    if let Some(value) = config.max_output_tokens {
        generation_config.max_output_tokens = Some(value);
    }
    if let Some(value) = config.media_resolution {
        generation_config.media_resolution = Some(value);
    }
    if let Some(value) = config.seed {
        generation_config.seed = Some(value);
    }
    if let Some(value) = config.speech_config.clone() {
        generation_config.speech_config = Some(value);
    }
    if let Some(value) = config.thinking_config.clone() {
        generation_config.thinking_config = Some(value);
    }
    if let Some(value) = config.enable_affective_dialog {
        generation_config.enable_affective_dialog = Some(value);
    }

    updated.then_some(generation_config)
}

fn normalize_model_name(model: &str) -> String {
    if model.starts_with("models/") {
        model.to_string()
    } else {
        format!("models/{model}")
    }
}

fn convert_bidi_setup_to_token_setup(
    mut body: Map<String, Value>,
    lock_additional_fields: Option<&Vec<String>>,
) -> Map<String, Value> {
    let setup = body
        .get("bidiGenerateContentSetup")
        .and_then(|value| value.as_object())
        .and_then(|value| value.get("setup"))
        .and_then(|value| value.as_object())
        .cloned();

    if let Some(setup_map) = setup {
        let field_mask = build_field_masks(&setup_map);
        body.insert(
            "bidiGenerateContentSetup".to_string(),
            Value::Object(setup_map),
        );

        match lock_additional_fields {
            None => {
                body.remove("fieldMask");
            }
            Some(additional_fields) if additional_fields.is_empty() => {
                if field_mask.is_empty() {
                    body.remove("fieldMask");
                } else {
                    body.insert("fieldMask".to_string(), Value::String(field_mask.join(",")));
                }
            }
            Some(additional_fields) => {
                let mut extra = Vec::new();
                for field in additional_fields {
                    extra.push(normalize_additional_field(field));
                }
                if field_mask.is_empty() && extra.is_empty() {
                    body.remove("fieldMask");
                } else if field_mask.is_empty() {
                    body.insert("fieldMask".to_string(), Value::String(extra.join(",")));
                } else if extra.is_empty() {
                    body.remove("fieldMask");
                } else {
                    body.insert(
                        "fieldMask".to_string(),
                        Value::String(format!("{},{}", field_mask.join(","), extra.join(","))),
                    );
                }
            }
        }
    } else if let Some(value) = body.get("fieldMask").cloned() {
        let list = parse_field_mask_list(&value);
        if list.is_empty() {
            body.remove("fieldMask");
        } else {
            body.insert("fieldMask".to_string(), Value::String(list.join(",")));
        }
    } else {
        body.remove("fieldMask");
    }

    if let Some(value) = body.get("bidiGenerateContentSetup") {
        let remove = match value {
            Value::Null => true,
            Value::Object(map) => map.is_empty(),
            _ => false,
        };
        if remove {
            body.remove("bidiGenerateContentSetup");
        }
    }

    body
}

fn build_field_masks(setup: &Map<String, Value>) -> Vec<String> {
    let mut fields = Vec::new();
    for (key, value) in setup {
        if let Value::Object(nested) = value {
            if !nested.is_empty() {
                for inner_key in nested.keys() {
                    fields.push(format!("{key}.{inner_key}"));
                }
                continue;
            }
        }
        fields.push(key.clone());
    }
    fields
}

fn parse_field_mask_list(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect(),
        Value::String(value) => value
            .split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn normalize_additional_field(field: &str) -> String {
    if field.contains('.') {
        return field.to_string();
    }
    let normalized = match field {
        "top_k" => "topK",
        "top_p" => "topP",
        "max_output_tokens" => "maxOutputTokens",
        "candidate_count" => "candidateCount",
        "response_logprobs" => "responseLogprobs",
        "response_mime_type" => "responseMimeType",
        "response_json_schema" => "responseJsonSchema",
        "response_modalities" => "responseModalities",
        "stop_sequences" => "stopSequences",
        "audio_timestamp" => "audioTimestamp",
        "presence_penalty" => "presencePenalty",
        "frequency_penalty" => "frequencyPenalty",
        "enable_enhanced_civic_answers" => "enableEnhancedCivicAnswers",
        "enable_affective_dialog" => "enableAffectiveDialog",
        "model_selection_config" => "modelSelectionConfig",
        "routing_config" => "routingConfig",
        _ => field,
    };

    if is_generation_config_field(normalized) {
        format!("generationConfig.{normalized}")
    } else {
        normalized.to_string()
    }
}

fn is_generation_config_field(field: &str) -> bool {
    const FIELDS: &[&str] = &[
        "temperature",
        "topP",
        "topK",
        "maxOutputTokens",
        "candidateCount",
        "seed",
        "responseLogprobs",
        "logprobs",
        "thinkingConfig",
        "speechConfig",
        "imageConfig",
        "mediaResolution",
        "responseMimeType",
        "responseSchema",
        "responseJsonSchema",
        "responseModalities",
        "stopSequences",
        "audioTimestamp",
        "presencePenalty",
        "frequencyPenalty",
        "enableEnhancedCivicAnswers",
        "enableAffectiveDialog",
        "modelSelectionConfig",
        "routingConfig",
    ];
    FIELDS.contains(&field)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Client;
    use rust_genai_types::config::{SpeechConfig, ThinkingConfig};
    use rust_genai_types::enums::{MediaResolution, Modality};
    use rust_genai_types::http::HttpOptions;
    use rust_genai_types::tokens::LiveConnectConstraints;
    use std::collections::HashMap;

    #[test]
    fn test_convert_bidi_setup_to_token_setup_field_mask() {
        let mut body = Map::new();
        body.insert(
            "bidiGenerateContentSetup".to_string(),
            json!({
                "setup": {
                    "model": "models/gemini-2.0-flash",
                    "generationConfig": { "temperature": 0.7, "topP": 0.9 }
                }
            }),
        );
        body.insert(
            "fieldMask".to_string(),
            json!(["topP", "response_modalities"]),
        );

        let result = convert_bidi_setup_to_token_setup(
            body,
            Some(&vec!["topP".to_string(), "response_modalities".to_string()]),
        );

        let field_mask = result
            .get("fieldMask")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        assert!(field_mask.contains("model"));
        assert!(field_mask.contains("generationConfig.temperature"));
        assert!(field_mask.contains("generationConfig.topP"));
    }

    #[test]
    fn test_convert_bidi_setup_to_token_setup_no_setup() {
        let mut body = Map::new();
        body.insert("fieldMask".to_string(), json!(["generationConfig.topP"]));
        let result = convert_bidi_setup_to_token_setup(
            body,
            Some(&vec!["generationConfig.topP".to_string()]),
        );
        assert_eq!(
            result
                .get("fieldMask")
                .and_then(|value| value.as_str())
                .unwrap_or(""),
            "generationConfig.topP"
        );
    }

    #[test]
    fn test_build_auth_tokens_url_with_overrides() {
        let client = Client::new("test-key").unwrap();
        let inner = client.models().inner;
        let options = HttpOptions {
            base_url: Some("https://example.com/".into()),
            api_version: Some("v99".into()),
            ..Default::default()
        };
        let url = build_auth_tokens_url(inner.as_ref(), Some(&options));
        assert_eq!(url, "https://example.com/v99/auth_tokens");
    }

    #[test]
    fn test_build_auth_token_body_with_constraints() {
        let config = CreateAuthTokenConfig {
            expire_time: Some("2025-01-01T00:00:00Z".into()),
            new_session_expire_time: None,
            uses: Some(2),
            live_connect_constraints: Some(LiveConnectConstraints {
                model: Some("gemini-2.0-flash".into()),
                config: Some(LiveConnectConfig {
                    response_modalities: Some(vec![Modality::Text]),
                    temperature: Some(0.4),
                    ..Default::default()
                }),
            }),
            lock_additional_fields: Some(vec!["model".into()]),
            http_options: None,
        };
        let body = build_auth_token_body(&config).unwrap();
        assert!(body.get("expireTime").is_some());
        assert!(body.get("bidiGenerateContentSetup").is_some());
        assert!(body.get("fieldMask").is_some());
    }

    #[test]
    fn test_build_auth_token_body_new_session_expire_time() {
        let config = CreateAuthTokenConfig {
            expire_time: None,
            new_session_expire_time: Some("2025-01-02T00:00:00Z".into()),
            uses: None,
            live_connect_constraints: None,
            lock_additional_fields: None,
            http_options: None,
        };
        let body = build_auth_token_body(&config).unwrap();
        assert!(body.get("newSessionExpireTime").is_some());
    }

    #[test]
    fn test_merge_extra_body_rejects_non_object() {
        let mut body = json!({});
        let options = HttpOptions {
            extra_body: Some(json!("bad")),
            ..Default::default()
        };
        let result = merge_extra_body(&mut body, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_http_options_invalid_header() {
        let http = reqwest::Client::new();
        let request = http.get("https://example.com");
        let mut headers = HashMap::new();
        headers.insert("bad header".into(), "value".into());
        let options = HttpOptions {
            headers: Some(headers),
            ..Default::default()
        };
        let result = apply_http_options(request, Some(&options));
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_http_options_invalid_header_value() {
        let http = reqwest::Client::new();
        let request = http.get("https://example.com");
        let mut headers = HashMap::new();
        headers.insert("x-test".into(), "bad\nvalue".into());
        let options = HttpOptions {
            headers: Some(headers),
            ..Default::default()
        };
        let result = apply_http_options(request, Some(&options));
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_http_options_with_timeout_and_headers() {
        let http = reqwest::Client::new();
        let request = http.get("https://example.com");
        let mut headers = HashMap::new();
        headers.insert("x-test".into(), "ok".into());
        let options = HttpOptions {
            timeout: Some(5),
            headers: Some(headers),
            ..Default::default()
        };
        let request = apply_http_options(request, Some(&options)).unwrap();
        let built = request.build().unwrap();
        assert!(built.headers().contains_key("x-test"));
    }

    #[test]
    fn test_merge_extra_body_object() {
        let mut body = json!({"a": 1});
        let options = HttpOptions {
            extra_body: Some(json!({"b": 2})),
            ..Default::default()
        };
        merge_extra_body(&mut body, &options).unwrap();
        assert_eq!(body["a"], 1);
        assert_eq!(body["b"], 2);
    }

    #[test]
    fn test_normalize_model_name() {
        assert_eq!(
            normalize_model_name("models/gemini-2.0-flash"),
            "models/gemini-2.0-flash"
        );
        assert_eq!(
            normalize_model_name("gemini-2.0-flash"),
            "models/gemini-2.0-flash"
        );
    }

    #[test]
    fn test_parse_field_mask_list_variants() {
        let list = parse_field_mask_list(&json!(["a", "", 1, "b"]));
        assert_eq!(list, vec!["a".to_string(), String::new(), "b".to_string()]);
        let list = parse_field_mask_list(&json!("a, ,b"));
        assert_eq!(list, vec!["a".to_string(), "b".to_string()]);
        let list = parse_field_mask_list(&json!(1));
        assert!(list.is_empty());
    }

    #[test]
    fn test_normalize_additional_field_variants() {
        assert_eq!(normalize_additional_field("top_p"), "generationConfig.topP");
        assert_eq!(normalize_additional_field("custom"), "custom");
        assert_eq!(
            normalize_additional_field("generationConfig.topP"),
            "generationConfig.topP"
        );
        assert_eq!(
            normalize_additional_field("response_mime_type"),
            "generationConfig.responseMimeType"
        );
    }

    #[test]
    fn test_build_field_masks_nested() {
        let mut setup = Map::new();
        setup.insert("model".to_string(), json!("models/x"));
        setup.insert(
            "generationConfig".to_string(),
            json!({"temperature": 0.2, "topP": 0.9}),
        );
        let fields = build_field_masks(&setup);
        assert!(fields.contains(&"model".to_string()));
        assert!(fields.contains(&"generationConfig.temperature".to_string()));
        assert!(fields.contains(&"generationConfig.topP".to_string()));
    }

    #[test]
    fn test_convert_bidi_setup_removes_field_mask_when_none() {
        let mut body = Map::new();
        body.insert(
            "bidiGenerateContentSetup".to_string(),
            json!({"setup": {"model": "models/gemini-2.0-flash"}}),
        );
        body.insert("fieldMask".to_string(), json!(["model"]));
        let result = convert_bidi_setup_to_token_setup(body, None);
        assert!(result.get("fieldMask").is_none());
    }

    #[test]
    fn test_convert_bidi_setup_string_field_mask() {
        let mut body = Map::new();
        body.insert("fieldMask".to_string(), json!("a, ,b"));
        let result = convert_bidi_setup_to_token_setup(body, Some(&vec![]));
        assert_eq!(
            result
                .get("fieldMask")
                .and_then(serde_json::Value::as_str)
                .unwrap(),
            "a,b"
        );
    }

    #[test]
    fn test_convert_bidi_setup_removes_empty_setup() {
        let mut body = Map::new();
        body.insert("bidiGenerateContentSetup".to_string(), Value::Null);
        let result = convert_bidi_setup_to_token_setup(body, Some(&vec![]));
        assert!(result.get("bidiGenerateContentSetup").is_none());
    }

    #[test]
    fn test_convert_bidi_setup_empty_field_mask_and_additional_fields() {
        let mut body = Map::new();
        body.insert("bidiGenerateContentSetup".to_string(), json!({"setup": {}}));
        body.insert("fieldMask".to_string(), json!(["model"]));
        let result = convert_bidi_setup_to_token_setup(body, Some(&vec![]));
        assert!(result.get("fieldMask").is_none());

        let mut body = Map::new();
        body.insert("bidiGenerateContentSetup".to_string(), json!({"setup": {}}));
        let result = convert_bidi_setup_to_token_setup(body, Some(&vec!["top_p".to_string()]));
        assert_eq!(
            result
                .get("fieldMask")
                .and_then(serde_json::Value::as_str)
                .unwrap(),
            "generationConfig.topP"
        );
    }

    #[test]
    fn test_convert_bidi_setup_removes_empty_field_mask_list() {
        let mut body = Map::new();
        body.insert("fieldMask".to_string(), json!(" , "));
        let result = convert_bidi_setup_to_token_setup(body, Some(&vec![]));
        assert!(result.get("fieldMask").is_none());
    }

    #[test]
    fn test_convert_bidi_setup_removes_empty_object() {
        let mut body = Map::new();
        body.insert("bidiGenerateContentSetup".to_string(), json!({}));
        let result = convert_bidi_setup_to_token_setup(body, Some(&vec![]));
        assert!(result.get("bidiGenerateContentSetup").is_none());
    }

    #[test]
    fn test_merge_generation_config_none() {
        let config = LiveConnectConfig::default();
        assert!(merge_generation_config(&config).is_none());
    }

    #[test]
    fn test_merge_generation_config_updates_fields() {
        let config = LiveConnectConfig {
            response_modalities: Some(vec![Modality::Text]),
            temperature: Some(0.4),
            top_p: Some(0.7),
            top_k: Some(2),
            max_output_tokens: Some(32),
            media_resolution: Some(MediaResolution::MediaResolutionHigh),
            seed: Some(7),
            speech_config: Some(SpeechConfig::default()),
            thinking_config: Some(ThinkingConfig::default()),
            enable_affective_dialog: Some(true),
            ..Default::default()
        };
        let merged = merge_generation_config(&config).unwrap();
        assert_eq!(merged.temperature, Some(0.4));
        assert_eq!(merged.top_p, Some(0.7));
        assert_eq!(merged.top_k, Some(2.0));
        assert_eq!(merged.max_output_tokens, Some(32));
        assert_eq!(
            merged.media_resolution,
            Some(MediaResolution::MediaResolutionHigh)
        );
        assert_eq!(merged.seed, Some(7));
        assert!(merged.speech_config.is_some());
        assert!(merged.thinking_config.is_some());
        assert_eq!(merged.enable_affective_dialog, Some(true));
        assert_eq!(merged.response_modalities, Some(vec![Modality::Text]));
    }

    #[test]
    fn test_build_live_setup_for_constraints() {
        let constraints = LiveConnectConstraints {
            model: Some("gemini-2.0-flash".into()),
            config: Some(LiveConnectConfig {
                response_modalities: Some(vec![Modality::Text]),
                ..Default::default()
            }),
        };
        let setup = build_live_setup_for_constraints(&constraints);
        assert_eq!(setup.model.as_deref(), Some("models/gemini-2.0-flash"));
        assert!(setup.generation_config.is_some());
    }

    #[test]
    fn test_ensure_gemini_backend_rejects_vertex() {
        let client = Client::new_vertex("proj", "loc").unwrap();
        let inner = client.models().inner;
        let err = ensure_gemini_backend(inner.as_ref()).unwrap_err();
        assert!(err.to_string().contains("only supported in Gemini API"));
    }
}
