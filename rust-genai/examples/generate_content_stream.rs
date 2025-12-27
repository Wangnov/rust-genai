use futures_util::StreamExt;
use rust_genai::types::content::Content;
use rust_genai::Client;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let mut stream = client
        .models()
        .generate_content_stream(
            "gemini-2.5-flash",
            vec![Content::text("用三句话介绍 Rust")],
            Default::default(),
        )
        .await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(text) = chunk.text() {
            print!("{text}");
            io::stdout().flush().ok();
        }
    }
    Ok(())
}
