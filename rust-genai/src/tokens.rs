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
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建 Live API 的 Ephemeral Token。
    pub async fn create(&self, mut config: CreateAuthTokenConfig) -> Result<AuthToken> {
        ensure_gemini_backend(&self.inner)?;

        let http_options = config.http_options.take();
        let mut body = build_auth_token_body(&config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }

        let url = build_auth_tokens_url(&self.inner, http_options.as_ref())?;
        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
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
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!("{base}{version}/auth_tokens"))
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
        let setup = build_live_setup_for_constraints(constraints)?;
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

fn build_live_setup_for_constraints(
    constraints: &LiveConnectConstraints,
) -> Result<LiveClientSetup> {
    let config = constraints.config.clone().unwrap_or_default();
    let generation_config = merge_generation_config(&config);
    let model = constraints
        .model
        .as_ref()
        .map(|value| normalize_model_name(value));

    Ok(LiveClientSetup {
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
    })
}

fn merge_generation_config(config: &LiveConnectConfig) -> Option<GenerationConfig> {
    let mut generation_config = config.generation_config.clone().unwrap_or_default();
    let mut updated = config.generation_config.is_some();

    if let Some(value) = config.response_modalities.clone() {
        generation_config.response_modalities = Some(value);
        updated = true;
    }
    if let Some(value) = config.temperature {
        generation_config.temperature = Some(value);
        updated = true;
    }
    if let Some(value) = config.top_p {
        generation_config.top_p = Some(value);
        updated = true;
    }
    if let Some(value) = config.top_k {
        generation_config.top_k = Some(value as f32);
        updated = true;
    }
    if let Some(value) = config.max_output_tokens {
        generation_config.max_output_tokens = Some(value);
        updated = true;
    }
    if let Some(value) = config.media_resolution {
        generation_config.media_resolution = Some(value);
        updated = true;
    }
    if let Some(value) = config.seed {
        generation_config.seed = Some(value);
        updated = true;
    }
    if let Some(value) = config.speech_config.clone() {
        generation_config.speech_config = Some(value);
        updated = true;
    }
    if let Some(value) = config.thinking_config.clone() {
        generation_config.thinking_config = Some(value);
        updated = true;
    }
    if let Some(value) = config.enable_affective_dialog {
        generation_config.enable_affective_dialog = Some(value);
        updated = true;
    }

    if updated {
        Some(generation_config)
    } else {
        None
    }
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
            .filter_map(|item| item.as_str().map(|value| value.to_string()))
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
}
