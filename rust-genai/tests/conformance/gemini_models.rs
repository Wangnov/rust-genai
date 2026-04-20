use crate::support::{build_live_gemini_client, live_gemini_model, setup_mock_gemini_context};
use rust_genai::tokenizer::SimpleTokenEstimator;
use rust_genai::types;
use rust_genai::types::content::Content;

#[tokio::test]
async fn mock_gemini_models_core_paths() {
    let ctx = setup_mock_gemini_context().await;
    let models = ctx.client.models();

    models.list().await.unwrap();
    models
        .list_with_config(types::models::ListModelsConfig {
            page_size: Some(1),
            ..Default::default()
        })
        .await
        .unwrap();
    models.get("gemini-3-flash-preview").await.unwrap();
    models
        .generate_content("gemini-3-flash-preview", vec![Content::text("hi")])
        .await
        .unwrap();
    models
        .embed_content("gemini-3-flash-preview", vec![Content::text("hi")])
        .await
        .unwrap();
    models
        .count_tokens("gemini-3-flash-preview", vec![Content::text("hi")])
        .await
        .unwrap();
    models
        .count_tokens_or_estimate(
            "gemini-3-flash-preview",
            vec![Content::text("hi")],
            types::models::CountTokensConfig::default(),
            Some(&SimpleTokenEstimator),
        )
        .await
        .unwrap();

    let chat = ctx.client.chats().create("gemini-3-flash-preview");
    chat.send_message("hello").await.unwrap();
}

#[tokio::test]
#[ignore = "live-gemini"]
async fn live_gemini_models_core_paths() {
    let client = build_live_gemini_client().unwrap();
    let model = live_gemini_model();

    let response = client
        .models()
        .generate_content(&model, vec![Content::text("Reply with exactly OK.")])
        .await
        .unwrap();
    assert!(response.text().is_some());

    let tokens = client
        .models()
        .count_tokens(&model, vec![Content::text("hello from rust conformance")])
        .await
        .unwrap();
    assert!(tokens.total_tokens.unwrap_or_default() > 0);
}
