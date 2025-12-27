use rust_genai::types::enums::SegmentMode;
use rust_genai::types::models::{Image, SegmentImageConfig, SegmentImageSource};
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    // Segment Image 仅支持 Vertex AI 后端。
    let client = Client::from_env()?;

    let source = SegmentImageSource {
        prompt: Some("foreground".to_string()),
        image: Some(Image {
            gcs_uri: Some("gs://your-bucket/input.jpg".to_string()),
            ..Default::default()
        }),
        scribble_image: None,
    };

    let config = SegmentImageConfig {
        mode: Some(SegmentMode::Foreground),
        ..Default::default()
    };

    let response = client
        .models()
        .segment_image("image-segmentation-001", source, config)
        .await?;

    println!("masks: {}", response.generated_masks.len());
    Ok(())
}
