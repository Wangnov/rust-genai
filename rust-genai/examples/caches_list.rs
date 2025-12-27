use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let caches = client.caches().list().await?;
    println!("{:?}", caches.cached_contents);
    Ok(())
}
