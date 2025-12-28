//! Live Music types.

use crate::base64_serde;
use serde::{Deserialize, Serialize};

/// Scale of the generated music.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Scale {
    #[default]
    ScaleUnspecified,
    CMajorAMinor,
    DFlatMajorBFlatMinor,
    DMajorBMinor,
    EFlatMajorCMinor,
    EMajorDFlatMinor,
    FMajorDMinor,
    GFlatMajorEFlatMinor,
    GMajorEMinor,
    AFlatMajorFMinor,
    AMajorGFlatMinor,
    BFlatMajorGMinor,
    BMajorAFlatMinor,
}

/// The mode of music generation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MusicGenerationMode {
    MusicGenerationModeUnspecified,
    Quality,
    Diversity,
    Vocalization,
}

/// The playback control signal to apply to the music generation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiveMusicPlaybackControl {
    PlaybackControlUnspecified,
    Play,
    Pause,
    Stop,
    ResetContext,
}

/// Message to be sent by the system when connecting to the API.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMusicClientSetup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Maps a prompt to a relative weight to steer music generation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WeightedPrompt {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
}

/// User input to start or steer the music.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMusicClientContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weighted_prompts: Option<Vec<WeightedPrompt>>,
}

/// Configuration for music generation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMusicGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bpm: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<Scale>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute_bass: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute_drums: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_bass_and_drums: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_generation_mode: Option<MusicGenerationMode>,
}

/// Messages sent by the client in the `LiveMusicClientMessage` call.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMusicClientMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup: Option<LiveMusicClientSetup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_content: Option<LiveMusicClientContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_generation_config: Option<LiveMusicGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_control: Option<LiveMusicPlaybackControl>,
}

/// Sent in response to a `LiveMusicClientSetup` message from the client.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMusicServerSetupComplete {}

/// Prompts and config used for generating this audio chunk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMusicSourceMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_content: Option<LiveMusicClientContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_generation_config: Option<LiveMusicGenerationConfig>,
}

/// Representation of an audio chunk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AudioChunk {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "base64_serde::option"
    )]
    pub data: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_metadata: Option<LiveMusicSourceMetadata>,
}

/// Server update generated by the model in response to client messages.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMusicServerContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_chunks: Option<Vec<AudioChunk>>,
}

/// A prompt that was filtered with the reason.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMusicFilteredPrompt {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtered_reason: Option<String>,
}

/// Response message for the `LiveMusicClientMessage` call.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveMusicServerMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_complete: Option<LiveMusicServerSetupComplete>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_content: Option<LiveMusicServerContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtered_prompt: Option<LiveMusicFilteredPrompt>,
}

impl LiveMusicServerMessage {
    /// 获取首个音频 chunk。
    #[must_use]
    pub fn first_audio_chunk(&self) -> Option<&AudioChunk> {
        self.server_content
            .as_ref()
            .and_then(|content| content.audio_chunks.as_ref())
            .and_then(|chunks| chunks.first())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn first_audio_chunk_returns_first() {
        let message = LiveMusicServerMessage {
            server_content: Some(LiveMusicServerContent {
                audio_chunks: Some(vec![
                    AudioChunk {
                        data: Some(vec![1, 2, 3]),
                        mime_type: Some("audio/wav".to_string()),
                        source_metadata: None,
                    },
                    AudioChunk {
                        data: Some(vec![4, 5]),
                        mime_type: None,
                        source_metadata: None,
                    },
                ]),
            }),
            ..Default::default()
        };

        let first = message.first_audio_chunk().unwrap();
        assert_eq!(first.mime_type.as_deref(), Some("audio/wav"));
    }

    #[test]
    fn audio_chunk_base64_roundtrip() {
        let chunk = AudioChunk {
            data: Some(vec![9, 8, 7]),
            mime_type: Some("audio/raw".to_string()),
            source_metadata: None,
        };
        let value = serde_json::to_value(&chunk).unwrap();
        assert_eq!(
            value,
            json!({
                "data": "CQgH",
                "mimeType": "audio/raw"
            })
        );

        let decoded: AudioChunk = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.data, Some(vec![9, 8, 7]));
    }
}
