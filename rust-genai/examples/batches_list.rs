use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let jobs = client.batches().list().await?;
    println!("{:?}", jobs.batch_jobs);
    Ok(())
}
