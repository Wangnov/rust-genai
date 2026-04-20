use crate::support::{build_live_vertex_client, live_vertex_model, setup_mock_vertex_context};
use rust_genai::types;
use rust_genai::types::content::Content;

#[tokio::test]
async fn mock_vertex_models_core_paths() {
    let ctx = setup_mock_vertex_context().await;
    let models = ctx.client.models();

    models.list().await.unwrap();
    models
        .get("publishers/google/models/gemini-3-flash-preview")
        .await
        .unwrap();
    models
        .generate_content(
            "publishers/google/models/gemini-3-flash-preview",
            vec![Content::text("hi")],
        )
        .await
        .unwrap();
    models
        .embed_content(
            "publishers/google/models/gemini-3-flash-preview",
            vec![Content::text("hi")],
        )
        .await
        .unwrap();
    models
        .count_tokens(
            "publishers/google/models/gemini-3-flash-preview",
            vec![Content::text("hi")],
        )
        .await
        .unwrap();
    models
        .compute_tokens(
            "publishers/google/models/gemini-3-flash-preview",
            vec![Content::text("hi")],
        )
        .await
        .unwrap();

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
#[ignore = "live-vertex"]
async fn live_vertex_models_core_paths() {
    let client = build_live_vertex_client().unwrap();
    let model = live_vertex_model();

    let response = client
        .models()
        .generate_content(&model, vec![Content::text("Reply with exactly OK.")])
        .await
        .unwrap();
    assert!(response.text().is_some());

    let count = client
        .models()
        .count_tokens(&model, vec![Content::text("vertex conformance")])
        .await
        .unwrap();
    assert!(count.total_tokens.unwrap_or_default() > 0);

    let compute = client
        .models()
        .compute_tokens(&model, vec![Content::text("vertex conformance")])
        .await
        .unwrap();
    assert!(compute
        .tokens_info
        .as_ref()
        .is_some_and(|items| !items.is_empty()));
}
