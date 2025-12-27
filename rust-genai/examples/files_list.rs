use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let files = client.files().list().await?;
    println!("{:?}", files.files);
    Ok(())
}
