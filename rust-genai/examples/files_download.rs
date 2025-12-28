use rust_genai::types::enums::FileSource;
use rust_genai::Client;
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

fn extension_from_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "image/bmp" => Some("bmp"),
        "image/tiff" => Some("tiff"),
        "audio/wav" | "audio/x-wav" => Some("wav"),
        "audio/pcm" => Some("pcm"),
        "video/mp4" => Some("mp4"),
        "application/pdf" => Some("pdf"),
        "application/json" => Some("json"),
        "text/plain" => Some("txt"),
        _ => None,
    }
}

fn sanitize_filename(name: &str) -> String {
    name.replace('/', "_")
}

fn ensure_extension(name: &str, mime: Option<&str>) -> String {
    if name.contains('.') {
        return name.to_string();
    }
    let ext = mime.and_then(extension_from_mime);
    ext.map_or_else(|| name.to_string(), |ext| format!("{name}.{ext}"))
}

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let file_name = if let Ok(value) = std::env::var("GENAI_FILE_NAME") {
        value
    } else {
        let files = client.files().all().await?;
        if let Some(name) = files
            .iter()
            .find(|file| file.source == Some(FileSource::Generated))
            .and_then(|file| file.name.clone())
        {
            name
        } else {
            println!("no generated files found; set GENAI_FILE_NAME to download a file.");
            return Ok(());
        }
    };
    if !file_name.starts_with("files/") {
        eprintln!("GENAI_FILE_NAME 需要是 File API 的名称（如 files/xxx），当前为：{file_name}");
        return Ok(());
    }
    let file = client.files().get(&file_name).await?;
    if file.source != Some(FileSource::Generated) {
        eprintln!("仅支持下载 GENERATED 文件，请先生成文件或改用自动选择逻辑。");
        return Ok(());
    }
    let bytes = client.files().download(&file_name).await?;
    let display_name = file
        .display_name
        .clone()
        .unwrap_or_else(|| sanitize_filename(&file_name));
    let base_name = ensure_extension(&sanitize_filename(&display_name), file.mime_type.as_deref());
    let output_dir = example_files_dir();
    std::fs::create_dir_all(&output_dir)?;
    let output_path = output_dir.join(base_name);
    std::fs::write(&output_path, &bytes)?;
    println!(
        "saved {} ({} bytes, 可用 GENAI_EXAMPLE_FILES_DIR 覆盖输出目录)",
        output_path.display(),
        bytes.len()
    );
    Ok(())
}
