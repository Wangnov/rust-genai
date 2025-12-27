use rust_genai::types::config::{GenerationConfig, PrebuiltVoiceConfig, SpeechConfig, VoiceConfig};
use rust_genai::types::enums::Modality;
use rust_genai::types::live_types::LiveConnectConfig;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;

    let generation_config = GenerationConfig {
        response_modalities: Some(vec![Modality::Audio]),
        speech_config: Some(SpeechConfig {
            voice_config: Some(VoiceConfig {
                prebuilt_voice_config: Some(PrebuiltVoiceConfig {
                    voice_name: Some("Kore".into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let config = LiveConnectConfig {
        generation_config: Some(generation_config),
        ..Default::default()
    };

    let mut session = client
        .live()
        .connect("gemini-2.5-flash-native-audio-preview-12-2025", config)
        .await?;

    session.send_text("请用中文朗读一句欢迎词。").await?;
    if let Some(message) = session.receive().await {
        println!("{:?}", message?);
    }
    session.close().await?;
    Ok(())
}
