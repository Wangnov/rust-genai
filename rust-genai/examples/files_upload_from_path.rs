use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let file = client.files().upload_from_path("README.md").await?;
    println!("uploaded: {:?}", file.name);
    Ok(())
}
