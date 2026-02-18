use rust_genai::types::models::{GenerateVideosConfig, GenerateVideosOperation};
use rust_genai::Client;
use std::path::{Path, PathBuf};

fn expand_tilde(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{stripped}");
        }
    }
    path.to_string()
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

fn output_video_path() -> PathBuf {
    if let Ok(path) = std::env::var("GENAI_VIDEO_PATH") {
        return PathBuf::from(expand_tilde(&path));
    }
    example_files_dir().join("generated_video.mp4")
}

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let operation: GenerateVideosOperation = if let Ok(name) = std::env::var("GENAI_OPERATION_NAME")
    {
        GenerateVideosOperation {
            name: Some(name),
            ..Default::default()
        }
    } else {
        let operation = client
            .models()
            .generate_videos_with_prompt(
                "veo-3.0-generate-001",
                "a city time lapse",
                GenerateVideosConfig::default(),
            )
            .await?;
        println!("operation: {:?}", operation.name);
        operation
    };

    let operation = client
        .operations()
        .wait_generate_videos_operation(operation)
        .await?;
    if let Some(error) = operation.error {
        eprintln!("video generation failed: {error:?}");
        return Ok(());
    }
    let Some(response) = operation.response else {
        eprintln!("operation response is empty");
        return Ok(());
    };
    let Some(video) = response
        .generated_videos
        .first()
        .and_then(|item| item.video.as_ref())
    else {
        if let Some(count) = response.rai_media_filtered_count {
            eprintln!("no video in response (filtered count: {count:?})");
        } else {
            eprintln!("no video in response");
        }
        return Ok(());
    };

    let output_path = output_video_path();
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    if let Some(bytes) = video.video_bytes.as_ref() {
        std::fs::write(&output_path, bytes)?;
        println!(
            "saved {} (可用 GENAI_EXAMPLE_FILES_DIR 覆盖输出目录)",
            output_path.display()
        );
        return Ok(());
    }
    if let Some(uri) = video.uri.as_ref() {
        if uri.contains("/files/") {
            let bytes = client.files().download(uri).await?;
            std::fs::write(&output_path, bytes)?;
            println!(
                "downloaded {} (可用 GENAI_EXAMPLE_FILES_DIR 覆盖输出目录)",
                output_path.display()
            );
            return Ok(());
        }
        let api_key = std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .unwrap_or_default();
        if api_key.is_empty() {
            eprintln!("video uri requires API key; set GEMINI_API_KEY to download");
            return Ok(());
        }
        let response = reqwest::Client::new()
            .get(uri)
            .header("x-goog-api-key", api_key)
            .send()
            .await?;
        let bytes = response.bytes().await?;
        std::fs::write(&output_path, &bytes)?;
        println!(
            "downloaded {} (可用 GENAI_EXAMPLE_FILES_DIR 覆盖输出目录)",
            output_path.display()
        );
        return Ok(());
    }

    eprintln!("video bytes/uri missing in response");
    Ok(())
}
