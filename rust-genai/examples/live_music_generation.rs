use rust_genai::types::live_music_types::{LiveMusicGenerationConfig, WeightedPrompt};
use rust_genai::{Client, Error};

fn client_from_env_v1alpha() -> rust_genai::Result<Client> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .or_else(|_| std::env::var("GOOGLE_API_KEY"))
        .map_err(|_| Error::InvalidConfig {
            message: "GEMINI_API_KEY or GOOGLE_API_KEY not found".into(),
        })?;
    Client::builder()
        .api_key(api_key)
        .api_version("v1alpha")
        .build()
}

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = client_from_env_v1alpha()?;
    let mut session = client.live_music().connect("lyria-realtime-exp").await?;

    session
        .set_weighted_prompts(vec![WeightedPrompt {
            text: Some("uplifting synthwave track".into()),
            weight: Some(1.0),
        }])
        .await?;

    session
        .set_music_generation_config(Some(LiveMusicGenerationConfig {
            bpm: Some(120),
            brightness: Some(0.6),
            density: Some(0.5),
            ..Default::default()
        }))
        .await?;

    session.play().await?;

    while let Some(message) = session.receive().await {
        let message = message?;
        if let Some(chunk) = message.first_audio_chunk() {
            let size = chunk.data.as_ref().map_or(0, Vec::len);
            println!("received audio chunk: {size} bytes");
        }
    }

    Ok(())
}
