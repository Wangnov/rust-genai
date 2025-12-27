use rust_genai::types::config::{
    GenerationConfig, MultiSpeakerVoiceConfig, PrebuiltVoiceConfig, SpeakerVoiceConfig,
    SpeechConfig, VoiceConfig,
};
use rust_genai::types::content::{Content, PartKind};
use rust_genai::types::enums::Modality;
use rust_genai::types::models::GenerateContentConfig;
use rust_genai::Client;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

fn example_files_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GENAI_EXAMPLE_FILES_DIR") {
        return PathBuf::from(dir);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("files")
        .join("output")
}

fn parse_sample_rate(mime_type: &str) -> u32 {
    const DEFAULT_RATE: u32 = 24_000;
    mime_type
        .split(';')
        .find_map(|part| part.trim().strip_prefix("rate="))
        .and_then(|rate| rate.parse().ok())
        .unwrap_or(DEFAULT_RATE)
}

fn write_wav(path: &Path, pcm_data: &[u8], sample_rate: u32) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_len = pcm_data.len() as u32;
    let chunk_size = 36 + data_len;

    file.write_all(b"RIFF")?;
    file.write_all(&chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_len.to_le_bytes())?;
    file.write_all(pcm_data)?;
    Ok(())
}

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let models = client.models();

    let speech_config = SpeechConfig {
        multi_speaker_voice_config: Some(MultiSpeakerVoiceConfig {
            speaker_voice_configs: Some(vec![
                SpeakerVoiceConfig {
                    speaker: Some("Alice".into()),
                    voice_config: Some(VoiceConfig {
                        prebuilt_voice_config: Some(PrebuiltVoiceConfig {
                            voice_name: Some("Kore".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                },
                SpeakerVoiceConfig {
                    speaker: Some("Bob".into()),
                    voice_config: Some(VoiceConfig {
                        prebuilt_voice_config: Some(PrebuiltVoiceConfig {
                            voice_name: Some("Puck".into()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                },
            ]),
        }),
        ..Default::default()
    };

    let generation_config = GenerationConfig {
        response_modalities: Some(vec![Modality::Audio]),
        speech_config: Some(speech_config),
        ..Default::default()
    };

    let config = GenerateContentConfig {
        generation_config: Some(generation_config),
        ..Default::default()
    };

    let response = models
        .generate_content_with_config(
            "gemini-2.5-flash-preview-tts",
            vec![Content::text("Alice: 你好！\nBob: 很高兴见到你。")],
            config,
        )
        .await?;

    let output_dir = example_files_dir();
    std::fs::create_dir_all(&output_dir)?;
    let mut audio_index = 0usize;

    for candidate in response.candidates {
        if let Some(content) = candidate.content {
            for part in content.parts {
                match part.kind {
                    PartKind::Text { text } => println!("{text}"),
                    PartKind::InlineData { inline_data } => {
                        if !inline_data.mime_type.starts_with("audio/") {
                            continue;
                        }
                        let filename =
                            output_dir.join(format!("tts_multispeaker_{audio_index}.wav"));
                        if inline_data.mime_type.contains("wav") {
                            std::fs::write(&filename, &inline_data.data)?;
                        } else {
                            let rate = parse_sample_rate(&inline_data.mime_type);
                            write_wav(&filename, &inline_data.data, rate)?;
                        }
                        println!(
                            "saved {} (可用 GENAI_EXAMPLE_FILES_DIR 覆盖输出目录)",
                            filename.display()
                        );
                        audio_index += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    if audio_index == 0 {
        println!("no audio data returned");
    }
    Ok(())
}
