use futures_util::StreamExt;

use crate::support::{build_live_gemini_client, live_gemini_model, setup_mock_gemini_context};
use rust_genai::types;
use rust_genai::types::content::Content;

#[tokio::test]
async fn mock_gemini_streaming_core_paths() {
    let ctx = setup_mock_gemini_context().await;
    let models = ctx.client.models();

    let mut stream = models
        .generate_content_stream(
            "gemini-3-flash-preview",
            vec![Content::text("hi")],
            types::models::GenerateContentConfig::default(),
        )
        .await
        .unwrap();
    let first = stream.next().await.unwrap().unwrap();
    assert_eq!(first.text().as_deref(), Some("ok"));

    let mut events = models
        .generate_content_event_stream(
            "gemini-3-flash-preview",
            vec![Content::text("hi")],
            types::models::GenerateContentConfig::default(),
        )
        .await
        .unwrap();
    let mut saw_done = false;
    while let Some(event) = events.next_event().await.unwrap() {
        if matches!(
            event,
            rust_genai::models::GenerateContentStreamEvent::Done(_)
        ) {
            saw_done = true;
            break;
        }
    }
    assert!(saw_done);
}

#[tokio::test]
#[ignore = "live-gemini"]
async fn live_gemini_streaming_core_paths() {
    let client = build_live_gemini_client().unwrap();
    let model = live_gemini_model();

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
