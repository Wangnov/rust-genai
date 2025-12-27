use rust_genai::types::batches::{BatchJobSource, CreateBatchJobConfig, InlinedRequest};
use rust_genai::types::content::Content;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let src = BatchJobSource {
        inlined_requests: Some(vec![InlinedRequest {
            model: Some("models/gemini-2.5-flash".into()),
            contents: Some(vec![Content::text("batch hello")]),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let job = client
        .batches()
        .create("gemini-2.5-flash", src, CreateBatchJobConfig::default())
        .await?;
    println!("batch job: {:?}", job.name);
    Ok(())
}
