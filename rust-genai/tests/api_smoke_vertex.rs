use futures_util::StreamExt;
use serde_json::json;
use wiremock::MockServer;

use rust_genai::types;
use rust_genai::{Backend, Client, Credentials};

mod support;
use support::mount_default_mock;

struct VertexTestContext {
    client: Client,
    _server: MockServer,
    _temp_dir: tempfile::TempDir,
}

async fn setup_vertex_context() -> VertexTestContext {
    let server = MockServer::start().await;
    mount_default_mock(&server).await;

    let dir = tempfile::tempdir().unwrap();
    let client_secret_path = dir.path().join("client_secret.json");
    let token_cache_path = dir.path().join("token.json");
    let token_uri = format!("{}/token", server.uri());
    std::fs::write(
        &client_secret_path,
        json!({
            "installed": {
                "client_id": "client-id",
                "client_secret": "client-secret",
                "token_uri": token_uri
            }
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        &token_cache_path,
        json!({
            "refresh_token": "refresh-1",
            "token_uri": format!("{}/token", server.uri())
        })
        .to_string(),
    )
    .unwrap();

    let client = Client::builder()
        .backend(Backend::VertexAi)
        .vertex_project("proj")
        .vertex_location("us-central1")
        .base_url(server.uri())
        .credentials(Credentials::OAuth {
            client_secret_path,
            token_cache_path: Some(token_cache_path),
        })
        .build()
        .unwrap();

    VertexTestContext {
        client,
        _server: server,
        _temp_dir: dir,
    }
}

#[tokio::test]
async fn api_smoke_vertex_models_text() {
    let ctx = setup_vertex_context().await;
    let client = &ctx.client;
    let models = client.models();

    models.list().await.unwrap();
    models
        .get("publishers/google/models/gemini-2.0-flash")
        .await
        .unwrap();
    models
        .generate_content(
            "publishers/google/models/gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
        )
        .await
        .unwrap();
    let mut stream = models
        .generate_content_stream(
            "publishers/google/models/gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
            types::models::GenerateContentConfig::default(),
        )
        .await
        .unwrap();
    stream.next().await.unwrap().unwrap();
    models
        .embed_content(
            "publishers/google/models/gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
        )
        .await
        .unwrap();
    models
        .count_tokens(
            "publishers/google/models/gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
        )
        .await
        .unwrap();
    models
        .compute_tokens(
            "publishers/google/models/gemini-2.0-flash",
            vec![types::content::Content::text("hi")],
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn api_smoke_vertex_models_media() {
    let ctx = setup_vertex_context().await;
    let client = &ctx.client;
    let models = client.models();

    models
        .generate_images(
            "publishers/google/models/imagen-3",
            "prompt",
            types::models::GenerateImagesConfig::default(),
        )
        .await
        .unwrap();
    models
        .generate_videos_with_prompt(
            "publishers/google/models/veo-1",
            "prompt",
            types::models::GenerateVideosConfig::default(),
        )
        .await
        .unwrap();
    models
        .edit_image(
            "publishers/google/models/image-model",
            "edit",
            vec![types::models::ReferenceImage {
                reference_image: Some(types::models::Image {
                    image_bytes: Some(vec![1, 2, 3]),
                    mime_type: Some("image/png".into()),
                    ..Default::default()
                }),
                reference_id: Some(1),
                ..Default::default()
            }],
            types::models::EditImageConfig::default(),
        )
        .await
        .unwrap();
    models
        .upscale_image(
            "publishers/google/models/image-model",
            types::models::Image {
                image_bytes: Some(vec![1, 2, 3]),
                mime_type: Some("image/png".into()),
                ..Default::default()
            },
            "x2",
            types::models::UpscaleImageConfig::default(),
        )
        .await
        .unwrap();
    models
        .recontext_image(
            "publishers/google/models/image-model",
            types::models::RecontextImageSource {
                prompt: Some("prompt".into()),
                ..Default::default()
            },
            types::models::RecontextImageConfig::default(),
        )
        .await
        .unwrap();
    models
        .segment_image(
            "publishers/google/models/image-model",
            types::models::SegmentImageSource {
                prompt: Some("segment".into()),
                ..Default::default()
            },
            types::models::SegmentImageConfig::default(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn api_smoke_vertex_caches() {
    let ctx = setup_vertex_context().await;
    let client = &ctx.client;
    let caches = client.caches();

    caches
        .create(
            "publishers/google/models/gemini-2.0-flash",
            types::caches::CreateCachedContentConfig {
                kms_key_name: Some("kms/key".into()),
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
async fn api_smoke_vertex_batches() {
    let ctx = setup_vertex_context().await;
    let client = &ctx.client;
    let batches = client.batches();

    batches
        .create(
            "publishers/google/models/gemini-2.0-flash",
            types::batches::BatchJobSource {
                format: Some("jsonl".into()),
                gcs_uri: Some(vec!["gs://input".into()]),
                ..Default::default()
            },
            types::batches::CreateBatchJobConfig {
                dest: Some(types::batches::BatchJobDestination {
                    format: Some("jsonl".into()),
                    gcs_uri: Some("gs://out".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    batches.get("batchPredictionJobs/1").await.unwrap();
    batches.list().await.unwrap();
    batches.all().await.unwrap();
    batches.delete("batchPredictionJobs/1").await.unwrap();
}

#[tokio::test]
async fn api_smoke_vertex_tunings_and_operations() {
    let ctx = setup_vertex_context().await;
    let client = &ctx.client;

    let tunings = client.tunings();
    tunings
        .tune(
            "publishers/google/models/gemini-2.0-flash",
            types::tunings::TuningDataset {
                gcs_uri: Some("gs://train".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    tunings.get("tuningJobs/1").await.unwrap();
    tunings.list().await.unwrap();
    tunings.all().await.unwrap();
    tunings.cancel("tuningJobs/1").await.unwrap();

    let operations = client.operations();
    operations.get("operations/1").await.unwrap();
    operations.list().await.unwrap();
    operations.all().await.unwrap();
}

#[tokio::test]
async fn api_smoke_vertex_gemini_only_rejections() {
    let ctx = setup_vertex_context().await;
    let client = &ctx.client;

    assert!(client.files().list().await.is_err());
    assert!(client.file_search_stores().list().await.is_err());
    assert!(client
        .interactions()
        .create(types::interactions::CreateInteractionConfig::new(
            "gemini-2.0-flash",
            "hello",
        ))
        .await
        .is_err());
}
