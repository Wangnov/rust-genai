use rust_genai::types::content::Content;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let response = client
        .models()
        .embed_content("text-embedding-004", vec![Content::text("hello")])
        .await?;
    println!("{:?}", response.embeddings);
    Ok(())
}
