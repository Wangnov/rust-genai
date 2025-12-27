use rust_genai::types::content::Content;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let result = client
        .models()
        .generate_content("invalid-model", vec![Content::text("hello")])
        .await;

    match result {
        Ok(resp) => println!("unexpected success: {:?}", resp.text()),
        Err(err) => eprintln!("request failed: {err}"),
    }

    Ok(())
}
