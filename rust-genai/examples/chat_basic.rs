use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let chat = client.chats().create("gemini-2.5-flash");
    let response = chat.send_message("你好").await?;
    println!("{:?}", response.text());
    Ok(())
}
