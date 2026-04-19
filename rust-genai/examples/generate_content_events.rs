use rust_genai::models::GenerateContentStreamEvent;
use rust_genai::types::content::Content;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let mut stream = client
        .models()
        .generate_content_event_stream(
            "gemini-2.5-flash-lite",
            vec![Content::text("用一句话介绍 Rust")],
            Default::default(),
        )
        .await?;

    while let Some(event) = stream.next_event().await? {
        match event {
            GenerateContentStreamEvent::Text(text) => println!("text: {text}"),
            GenerateContentStreamEvent::FunctionCall(call) => println!("tool: {:?}", call.name),
            GenerateContentStreamEvent::Usage(usage) => {
                println!("usage: {:?}", usage.total_token_count)
            }
            GenerateContentStreamEvent::Response(_) => {}
            GenerateContentStreamEvent::Done(_) => println!("done"),
        }
    }

    Ok(())
}
