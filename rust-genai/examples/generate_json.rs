use rust_genai::types::content::Content;
use rust_genai::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Greeting {
    message: String,
}

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let greeting = client
        .models()
        .generate_json::<Greeting>(
            "gemini-2.5-flash-lite",
            vec![Content::text(
                "Return JSON with one field named `message` containing a short greeting.",
            )],
        )
        .await?;
    println!("{}", greeting.message);
    Ok(())
}
