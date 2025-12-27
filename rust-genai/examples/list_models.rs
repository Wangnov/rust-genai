use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let models = client.models().list().await?;
    println!("{:?}", models.models);
    Ok(())
}
