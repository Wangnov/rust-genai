use rust_genai::types::config::GenerationConfig;
use rust_genai::types::content::{Content, PartKind};
use rust_genai::types::enums::Modality;
use rust_genai::types::models::GenerateContentConfig;
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
    let config = GenerateContentConfig {
        generation_config: Some(GenerationConfig {
            response_modalities: Some(vec![Modality::Text, Modality::Image]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let response = client
        .models()
        .generate_content_with_config(
            "gemini-2.5-flash-image",
            vec![Content::text(
                "Create a picture of a nano banana dish in a fancy restaurant with a Gemini theme",
            )],
            config,
        )
        .await?;

    let mut image_index = 0usize;
    let output_dir = example_files_dir();
    std::fs::create_dir_all(&output_dir)?;

    for candidate in response.candidates {
        if let Some(content) = candidate.content {
            for part in content.parts {
                match part.kind {
                    PartKind::Text { text } => println!("{text}"),
                    PartKind::InlineData { inline_data } => {
                        let ext = extension_from_mime(&inline_data.mime_type);
                        let filename =
                            output_dir.join(format!("gemini_native_image_{image_index}.{ext}"));
                        std::fs::write(&filename, inline_data.data)?;
                        println!(
                            "saved {} (可用 GENAI_EXAMPLE_FILES_DIR 覆盖输出目录)",
                            filename.display()
                        );
                        image_index += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    if image_index == 0 {
        println!("no image data returned");
    }

    Ok(())
}
