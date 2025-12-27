use rust_genai::types::caches::CreateCachedContentConfig;
use rust_genai::types::content::Content;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let long_text = "hello ".repeat(2000);
    let config = CreateCachedContentConfig {
        display_name: Some("demo-cache".into()),
        contents: Some(vec![Content::text(long_text)]),
        ..Default::default()
    };
    let cache = client.caches().create("gemini-2.5-flash", config).await?;
    println!("cache: {:?}", cache.name);
    Ok(())
}
