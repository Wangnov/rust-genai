use rust_genai::types::file_search_stores::UploadToFileSearchStoreConfig;
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let store_name = if let Ok(value) = std::env::var("GENAI_FILE_SEARCH_STORE") {
        value
    } else {
        let stores = client.file_search_stores().all().await?;
        if let Some(name) = stores.first().and_then(|store| store.name.clone()) {
            name
        } else {
            println!("no file search store found; set GENAI_FILE_SEARCH_STORE to upload.");
            return Ok(());
        }
    };
    let op = client
        .file_search_stores()
        .upload_to_file_search_store_from_path(
            &store_name,
            "README.md",
            UploadToFileSearchStoreConfig::default(),
        )
        .await?;
    println!("operation: {:?}", op.name);
    Ok(())
}
