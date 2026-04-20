use rust_genai::types;

use crate::support::setup_mock_vertex_context;

#[tokio::test]
async fn mock_vertex_guards_core_paths() {
    let ctx = setup_mock_vertex_context().await;
    let client = &ctx.client;

    assert!(client.files().list().await.is_err());
    assert!(client.file_search_stores().list().await.is_err());
    assert!(client
        .documents()
        .list("documentStores/store1")
        .await
        .is_err());
    assert!(client
        .interactions()
        .create(types::interactions::CreateInteractionConfig::new(
            "gemini-3-flash-preview",
            "hello",
        ))
        .await
        .is_err());
    assert!(client.deep_research().start("question").await.is_err());
}
