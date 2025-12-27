use rust_genai::types::interactions::{CreateInteractionConfig, InteractionInput};
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let interactions = client.interactions();

    let input = InteractionInput::text("用一句话介绍 Rust 的优势。");
    let config = CreateInteractionConfig::new("gemini-2.5-flash", input);
    let interaction = interactions.create(config).await?;

    println!("{:#?}", interaction.outputs);
    Ok(())
}
