use futures_util::StreamExt;
use wiremock::MockServer;

use rust_genai::files::WaitForFileConfig;
use rust_genai::tokenizer::SimpleTokenEstimator;
use rust_genai::types;
use rust_genai::Client;

mod support;
use support::mount_default_mock;

struct GeminiTestContext {
    client: Client,
    _server: MockServer,
}

async fn setup_gemini_context() -> GeminiTestContext {
    let server = MockServer::start().await;
    mount_default_mock(&server).await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();

    GeminiTestContext {
        client,
        _server: server,
    }
}

#[tokio::test]
async fn api_smoke_gemini_models_text() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let models = client.models();

    models.list().await.unwrap();
    models
        .list_with_config(types::models::ListModelsConfig {
            page_size: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    models.get("gemini-2.0-flash").await.unwrap();
    models
        .generate_content(
            "gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
        )
        .await
        .unwrap();
    let mut stream = models
        .generate_content_stream(
            "gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
            types::models::GenerateContentConfig::default(),
        )
        .await
        .unwrap();
    stream.next().await.unwrap().unwrap();
    models
        .embed_content(
            "gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
        )
        .await
        .unwrap();
    models
        .count_tokens(
            "gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
        )
        .await
        .unwrap();
    models
        .count_tokens_or_estimate(
            "gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
            types::models::CountTokensConfig::default(),
            Some(&SimpleTokenEstimator),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_models_media() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let models = client.models();

    models
        .generate_images(
            "imagen-3",
            "prompt",
            types::models::GenerateImagesConfig::default(),
        )
        .await
        .unwrap();
    models
        .generate_videos_with_prompt(
            "veo-1",
            "prompt",
            types::models::GenerateVideosConfig::default(),
        )
        .await
        .unwrap();
    models
        .update(
            "models/gemini-2.0-flash",
            types::models::UpdateModelConfig {
                display_name: Some("updated".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    models
        .delete(
            "models/gemini-2.0-flash",
            types::models::DeleteModelConfig::default(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_chats() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let chat = client.chats().create("gemini-2.0-flash");
    chat.send_message("hello").await.unwrap();
    let chat_stream = chat.send_message_stream("hi").await.unwrap();
    let mut chat_stream = Box::pin(chat_stream);
    chat_stream.next().await.unwrap().unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_files() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let files = client.files();

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
}

#[tokio::test]
async fn api_smoke_gemini_caches() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let caches = client.caches();

    caches
        .create(
            "gemini-2.0-flash",
            types::caches::CreateCachedContentConfig {
                contents: Some(vec![types::content::Content::text("cache")]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    caches.get("cachedContents/1").await.unwrap();
    caches
        .update(
            "cachedContents/1",
            types::caches::UpdateCachedContentConfig {
                ttl: Some("3600s".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    caches.delete("cachedContents/1").await.unwrap();
    caches.list().await.unwrap();
    caches.all().await.unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_batches() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let batches = client.batches();

    batches
        .create(
            "gemini-2.0-flash",
            types::batches::BatchJobSource {
                inlined_requests: Some(vec![types::batches::InlinedRequest {
                    model: Some("models/gemini-2.0-flash".into()),
                    contents: Some(vec![types::content::Content::text("hi")]),
                    metadata: None,
                    config: None,
                }]),
                ..Default::default()
            },
            types::batches::CreateBatchJobConfig::default(),
        )
        .await
        .unwrap();
    batches.get("batches/1").await.unwrap();
    batches.delete("batches/1").await.unwrap();
    batches.list().await.unwrap();
    batches.all().await.unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_documents() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let docs = client.documents();

    docs.get("fileSearchStores/store1/documents/doc1")
        .await
        .unwrap();
    docs.delete("fileSearchStores/store1/documents/doc1")
        .await
        .unwrap();
    docs.list("documentStores/store1").await.unwrap();
    docs.all("documentStores/store1").await.unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_file_search_stores() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let stores = client.file_search_stores();

    stores
        .create(types::file_search_stores::CreateFileSearchStoreConfig {
            display_name: Some("store".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    stores.get("fileSearchStores/1").await.unwrap();
    stores.delete("fileSearchStores/1").await.unwrap();
    stores.list().await.unwrap();
    stores.all().await.unwrap();
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

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("sample.txt");
    tokio::fs::write(&file_path, "hi").await.unwrap();
    stores
        .upload_to_file_search_store_from_path(
            "fileSearchStores/1",
            &file_path,
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
}

#[tokio::test]
async fn api_smoke_gemini_operations() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let operations = client.operations();
    operations.get("operations/1").await.unwrap();
    operations.list().await.unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_interactions() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let interactions = client.interactions();

    interactions
        .create(types::interactions::CreateInteractionConfig::new(
            "gemini-2.0-flash",
            "hello",
        ))
        .await
        .unwrap();
    let mut interaction_stream = interactions
        .create_stream(types::interactions::CreateInteractionConfig::new(
            "gemini-2.0-flash",
            "hello",
        ))
        .await
        .unwrap();
    interaction_stream.next().await.unwrap().unwrap();
    interactions.get("interactions/1").await.unwrap();
    interactions.delete("interactions/1").await.unwrap();
    interactions.cancel("interactions/1").await.unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_deep_research() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let deep_research = client.deep_research();

    deep_research
        .start("gemini-2.0-flash", "question")
        .await
        .unwrap();
    let mut deep_stream = deep_research
        .stream_with_config(types::interactions::CreateInteractionConfig::new(
            "gemini-2.0-flash",
            "question",
        ))
        .await
        .unwrap();
    deep_stream.next().await.unwrap().unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_auth_tokens() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let tokens = client.auth_tokens();
    tokens
        .create(types::tokens::CreateAuthTokenConfig::default())
        .await
        .unwrap();
}

#[tokio::test]
async fn api_smoke_gemini_tunings() {
    let ctx = setup_gemini_context().await;
    let client = &ctx.client;
    let tunings = client.tunings();

    tunings
        .tune(
            "gemini-2.0-flash",
            types::tunings::TuningDataset {
                examples: Some(vec![types::tunings::TuningExample {
                    text_input: Some("hi".into()),
                    output: Some("ok".into()),
                }]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    tunings.get("tuningJobs/1").await.unwrap();
    tunings.list().await.unwrap();
    tunings.all().await.unwrap();
    tunings.cancel("tuningJobs/1").await.unwrap();
}
