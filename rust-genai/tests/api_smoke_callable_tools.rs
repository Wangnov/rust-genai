use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use rust_genai::types;
use rust_genai::Client;

#[tokio::test]
async fn generate_content_with_callable_tools_flow() {
    let server = MockServer::start().await;
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(move |_req: &Request| {
            let call_index = counter_clone.fetch_add(1, Ordering::SeqCst);
            if call_index == 0 {
                ResponseTemplate::new(200).set_body_json(json!({
                    "candidates": [
                        {"content": {"role": "model", "parts": [
                            {"functionCall": {"name": "sum", "args": {"a": 1, "b": 2}}}
                        ]}}
                    ]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(json!({
                    "candidates": [
                        {"content": {"role": "model", "parts": [{"text": "done"}]}}
                    ]
                }))
            }
        })
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();

    let mut tool = rust_genai::afc::InlineCallableTool::from_declarations(vec![
        types::tool::FunctionDeclaration {
            name: "sum".into(),
            description: None,
            parameters: None,
            parameters_json_schema: None,
            response: None,
            response_json_schema: None,
            behavior: None,
        },
    ]);
    tool.register_handler("sum", |value| async move {
        let a = value["a"].as_i64().unwrap_or(0);
        let b = value["b"].as_i64().unwrap_or(0);
        Ok(json!({ "result": a + b }))
    });

    let response = client
        .models()
        .generate_content_with_callable_tools(
            "gemini-2.0-flash",
            vec![types::content::Content::text("calc")],
            types::models::GenerateContentConfig::default(),
            vec![Box::new(tool)],
        )
        .await
        .unwrap();

    assert_eq!(response.text().as_deref(), Some("done"));
}
