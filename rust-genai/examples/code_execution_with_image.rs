use rust_genai::types::content::{Content, Part};
use rust_genai::types::models::GenerateContentConfig;
use rust_genai::types::tool::{CodeExecution, Tool};
use rust_genai::Client;
use std::path::Path;

fn guess_mime_type(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "tiff" | "tif" => Some("image/tiff"),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let models = client.models();

    let tools = vec![Tool {
        code_execution: Some(CodeExecution::default()),
        ..Default::default()
    }];

    let image_path = match std::env::var("GENAI_IMAGE_PATH") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("请设置 GENAI_IMAGE_PATH 指向本地图片文件。");
            return Ok(());
        }
    };
    let image_path = Path::new(&image_path);
    let mime_type = match std::env::var("GENAI_IMAGE_MIME") {
        Ok(value) => value,
        Err(_) => match guess_mime_type(image_path) {
            Some(value) => value.to_string(),
            None => {
                eprintln!("无法推断图片 MIME，请设置 GENAI_IMAGE_MIME（如 image/png）。");
                return Ok(());
            }
        },
    };
    let image_bytes = std::fs::read(image_path)?;
    let image_part = Part::inline_data(image_bytes, mime_type);
    let prompt = Part::text("请分析这张图并用代码计算主色值。");
    let contents = vec![Content::from_parts(
        vec![image_part, prompt],
        rust_genai::types::content::Role::User,
    )];

    let config = GenerateContentConfig {
        tools: Some(tools),
        ..Default::default()
    };

    let response = models
        .generate_content_with_config("gemini-3-flash-preview", contents, config)
        .await?;

    println!("{:#?}", response.candidates);
    Ok(())
}
