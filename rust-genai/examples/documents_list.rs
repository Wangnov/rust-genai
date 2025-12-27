use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let store = match std::env::var("GENAI_FILE_SEARCH_STORE") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("请设置 GENAI_FILE_SEARCH_STORE（如 fileSearchStores/xxx）。");
            return Ok(());
        }
    };
    let docs = client.documents().list(store).await?;
    println!("{:?}", docs.documents);
    Ok(())
}
