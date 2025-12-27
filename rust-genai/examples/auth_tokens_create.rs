use rust_genai::types::http::HttpOptions;
use rust_genai::types::tokens::{CreateAuthTokenConfig, LiveConnectConstraints};
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let config = CreateAuthTokenConfig {
        http_options: Some(HttpOptions {
            api_version: Some("v1alpha".into()),
            ..Default::default()
        }),
        live_connect_constraints: Some(LiveConnectConstraints {
            model: Some("gemini-2.5-flash".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let token = client.auth_tokens().create(config).await?;
    println!("token: {:?}", token.name);
    Ok(())
}
