use rust_genai::types::models::GenerateImagesConfig;
use rust_genai::Client;
use std::path::{Path, PathBuf};

fn extension_from_mime(mime: &str) -> &str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        _ => "bin",
    }
}

fn example_files_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GENAI_EXAMPLE_FILES_DIR") {
        return PathBuf::from(dir);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("files")
        .join("output")
}

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let model = std::env::var("GENAI_IMAGE_MODEL")
        .unwrap_or_else(|_| "imagen-4.0-generate-001".to_string());
    let config = GenerateImagesConfig {
        number_of_images: Some(1),
        ..Default::default()
    };
    let response = client
        .models()
        .generate_images(model, "a red fox in snow", config)
        .await?;
    if response.generated_images.is_empty() {
        println!("no images generated");
        return Ok(());
    }

    let output_dir = example_files_dir();
    std::fs::create_dir_all(&output_dir)?;

    for (idx, generated) in response.generated_images.iter().enumerate() {
        let Some(image) = generated.image.as_ref() else {
            continue;
        };
        let Some(bytes) = image.image_bytes.as_ref() else {
            continue;
        };
        let ext = image
            .mime_type
            .as_deref()
            .map(extension_from_mime)
            .unwrap_or("png");
        let filename = output_dir.join(format!("generated_image_{idx}.{ext}"));
        std::fs::write(&filename, bytes)?;
        println!(
            "saved {} (可用 GENAI_EXAMPLE_FILES_DIR 覆盖输出目录)",
            filename.display()
        );
    }
    Ok(())
}
