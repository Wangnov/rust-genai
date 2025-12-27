use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let chat = client.chats().create("gemini-2.5-flash");
    chat.send_message("第一条消息").await?;
    chat.send_message("第二条消息").await?;
    let history = chat.history().await;
    println!("history size: {}", history.len());
    Ok(())
}
