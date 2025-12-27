use futures_util::StreamExt;
use rust_genai::Client;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let chat = client.chats().create("gemini-2.5-flash");
    let stream = chat.send_message_stream("写一句开场白").await?;
    futures_util::pin_mut!(stream);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(text) = chunk.text() {
            print!("{text}");
            io::stdout().flush().ok();
        }
    }
    Ok(())
}
