use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::base64_serde;
use crate::enums::{
    FeatureSelectionPreference, HarmBlockMethod, HarmBlockThreshold, HarmCategory, MediaResolution,
    Modality, ThinkingLevel,
};
use crate::tool::Schema;

/// 生成配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    /// 是否返回 logprobs 结果。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_logprobs: Option<bool>,
    /// 每个 token 返回的候选数（0-20）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech_config: Option<SpeechConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_config: Option<ImageConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_resolution: Option<MediaResolution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_json_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_modalities: Option<Vec<Modality>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_timestamp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_enhanced_civic_answers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_affective_dialog: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_selection_config: Option<ModelSelectionConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_config: Option<GenerationConfigRoutingConfig>,
}

/// 安全设置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafetySetting {
    pub category: HarmCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<HarmBlockThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<HarmBlockMethod>,
}

/// Thinking 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<ThinkingLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_thoughts: Option<bool>,
}

/// 语音合成配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpeechConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_config: Option<VoiceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_speaker_voice_config: Option<MultiSpeakerVoiceConfig>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VoiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicated_voice_config: Option<ReplicatedVoiceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prebuilt_voice_config: Option<PrebuiltVoiceConfig>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplicatedVoiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "base64_serde::option"
    )]
    pub voice_sample_audio: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrebuiltVoiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_name: Option<String>,
    /// Forward-compatible extension fields.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerVoiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_config: Option<VoiceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MultiSpeakerVoiceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker_voice_configs: Option<Vec<SpeakerVoiceConfig>>,
}

/// 图像生成配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImageConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_generation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression_quality: Option<i32>,
}

/// 模型选择配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelectionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_selection_preference: Option<FeatureSelectionPreference>,
}

/// 路由配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfigRoutingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_routing_mode: Option<GenerationConfigRoutingConfigAutoRoutingMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_routing_mode: Option<GenerationConfigRoutingConfigManualRoutingMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfigRoutingConfigAutoRoutingMode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_routing_preference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfigRoutingConfigManualRoutingMode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::{MediaResolution, Modality, ThinkingLevel};
    use crate::tool::Schema;

    #[test]
    fn generation_config_serializes_camel_case() {
        let config = GenerationConfig {
            temperature: Some(0.7),
            max_output_tokens: Some(128),
            response_mime_type: Some("application/json".into()),
            response_modalities: Some(vec![Modality::Text, Modality::Audio]),
            media_resolution: Some(MediaResolution::MediaResolutionHigh),
            response_schema: Some(Schema::string()),
            thinking_config: Some(ThinkingConfig {
                thinking_level: Some(ThinkingLevel::High),
                include_thoughts: Some(true),
                thinking_budget: None,
            }),
            ..Default::default()
        };

        let value = serde_json::to_value(&config).unwrap();
        assert!(value.get("maxOutputTokens").is_some());
        assert!(value.get("responseMimeType").is_some());
        assert!(value.get("responseModalities").is_some());
        assert!(value.get("mediaResolution").is_some());
        assert!(value.get("thinkingConfig").is_some());
    }

    #[test]
    fn safety_setting_roundtrip() {
        let setting = SafetySetting {
            category: HarmCategory::HarmCategoryHarassment,
            threshold: Some(HarmBlockThreshold::BlockOnlyHigh),
            method: None,
        };
        let json = serde_json::to_string(&setting).unwrap();
        let decoded: SafetySetting = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.category, HarmCategory::HarmCategoryHarassment);
    }
}
