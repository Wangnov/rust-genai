use crate::support::{
    build_live_gemini_client, expensive_opt_in, live_gemini_model, setup_mock_gemini_context,
};
use rust_genai::files::WaitForFileConfig;
use rust_genai::types;

#[tokio::test]
async fn mock_gemini_files_core_paths() {
    let ctx = setup_mock_gemini_context().await;
    let files = ctx.client.files();

    files.list().await.unwrap();
    files.all().await.unwrap();
    files.get("files/abc").await.unwrap();
    files.download("files/abc").await.unwrap();
    files.delete("files/abc").await.unwrap();
    files.upload(b"abc".to_vec(), "text/plain").await.unwrap();

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("sample.txt");
    tokio::fs::write(&file_path, "hi").await.unwrap();
    files.upload_from_path(&file_path).await.unwrap();
    files
        .wait_for_active("files/abc", WaitForFileConfig::default())
        .await
        .unwrap();

    let stores = ctx.client.file_search_stores();
    stores.list().await.unwrap();
    stores.all().await.unwrap();
    stores.get("fileSearchStores/1").await.unwrap();
    stores.delete("fileSearchStores/1").await.unwrap();
    stores
        .create(types::file_search_stores::CreateFileSearchStoreConfig {
            display_name: Some("store".into()),
            ..Default::default()
        })
        .await
        .unwrap();

    let upload_path = dir.path().join("search-store.txt");
    tokio::fs::write(&upload_path, "search").await.unwrap();
    stores
        .upload_to_file_search_store(
            "fileSearchStores/1",
            b"content".to_vec(),
            types::file_search_stores::UploadToFileSearchStoreConfig {
                mime_type: Some("text/plain".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    stores
        .upload_to_file_search_store_from_path(
            "fileSearchStores/1",
            &upload_path,
            types::file_search_stores::UploadToFileSearchStoreConfig {
                mime_type: Some("text/plain".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    stores
        .import_file(
            "fileSearchStores/1",
            "files/abc",
            types::file_search_stores::ImportFileConfig::default(),
        )
        .await
        .unwrap();

    let documents = ctx.client.documents();
    documents
        .get("fileSearchStores/store1/documents/doc1")
        .await
        .unwrap();
    documents
        .delete("fileSearchStores/store1/documents/doc1")
        .await
        .unwrap();
    documents.list("documentStores/store1").await.unwrap();
    documents.all("documentStores/store1").await.unwrap();
}

#[tokio::test]
#[ignore = "expensive"]
async fn expensive_live_gemini_files_core_paths() {
    if !expensive_opt_in() {
        eprintln!(
            "Skipping expensive Gemini file probe. Set GENAI_CONFORMANCE_ENABLE_EXPENSIVE=1."
        );
        return;
    }

    let client = build_live_gemini_client().unwrap();
    let _ = live_gemini_model();
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("conformance-upload.txt");
    tokio::fs::write(&file_path, "rust-genai conformance upload")
        .await
        .unwrap();

    let file = client.files().upload_from_path(&file_path).await.unwrap();
    let name = file.name.expect("uploaded file name");
    client.files().get(&name).await.unwrap();
    client.files().delete(&name).await.unwrap();
}
