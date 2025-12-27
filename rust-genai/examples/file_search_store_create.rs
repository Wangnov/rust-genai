use rust_genai::types::file_search_stores::CreateFileSearchStoreConfig;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let config = CreateFileSearchStoreConfig {
        display_name: Some("demo-store".into()),
        ..Default::default()
    };
    let store = client.file_search_stores().create(config).await?;
    println!("store: {:?}", store.name);
    Ok(())
}
