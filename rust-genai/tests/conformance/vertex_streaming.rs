use futures_util::StreamExt;

use crate::support::{build_live_vertex_client, live_vertex_model, setup_mock_vertex_context};
use rust_genai::types;
use rust_genai::types::content::Content;

#[tokio::test]
async fn mock_vertex_streaming_core_paths() {
    let ctx = setup_mock_vertex_context().await;
    let models = ctx.client.models();

    let mut stream = models
        .generate_content_stream(
            "publishers/google/models/gemini-3-flash-preview",
            vec![Content::text("hi")],
            types::models::GenerateContentConfig::default(),
        )
        .await
        .unwrap();
    let first = stream.next().await.unwrap().unwrap();
    assert_eq!(first.text().as_deref(), Some("ok"));
}

#[tokio::test]
#[ignore = "live-vertex"]
async fn live_vertex_streaming_core_paths() {
    let client = build_live_vertex_client().unwrap();
    let model = live_vertex_model();

    let mut stream = client
        .models()
        .generate_content_stream(
            &model,
            vec![Content::text("Reply with exactly three lowercase words.")],
            types::models::GenerateContentConfig::default(),
        )
        .await
        .unwrap();

    let mut chunks = 0usize;
    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        chunks += 1;
        if let Some(delta) = chunk.text() {
            text.push_str(&delta);
        }
    }

    assert!(chunks > 0);
    assert!(!text.trim().is_empty());
}
