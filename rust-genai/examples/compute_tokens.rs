use rust_genai::types::content::Content;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    // Vertex AI only.
    let client = Client::new_vertex("YOUR_PROJECT", "us-central1")?;
    let response = client
        .models()
        .compute_tokens("gemini-2.5-flash", vec![Content::text("hello")])
        .await?;
    println!("{:?}", response.tokens_info);
    Ok(())
}
