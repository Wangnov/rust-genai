use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .or_else(|_| std::env::var("GOOGLE_API_KEY"))
        .unwrap_or_else(|_| "YOUR_API_KEY".to_string());

    let mut builder = Client::builder().api_key(api_key).timeout(30);
    if let Ok(proxy) = std::env::var("GENAI_PROXY") {
        builder = builder.proxy(proxy);
    }

    let client = builder.build()?;

    let models = client.models().list().await?;
    println!("{:?}", models.models);
    Ok(())
}
