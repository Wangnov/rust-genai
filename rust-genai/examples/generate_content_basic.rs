use rust_genai::types::content::Content;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let response = client
        .models()
        .generate_content("gemini-2.5-flash", vec![Content::text("给我一句问候")])
        .await?;
    println!("{:?}", response.text());
    Ok(())
}
