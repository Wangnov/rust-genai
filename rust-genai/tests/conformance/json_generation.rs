use serde::Deserialize;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::config::GenerationConfig;
use rust_genai::types::content::Content;
use rust_genai::types::models::GenerateContentConfig;
use rust_genai::Client;

#[derive(Debug, Deserialize)]
struct JsonSmokeResponse {
    answer: String,
}

#[tokio::test]
async fn mock_json_generation_parses_structured_output() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.5-flash-lite:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "{\"answer\":\"ok\"}"}
                    ]
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();

    let response = client
        .models()
        .generate_json::<JsonSmokeResponse>(
            "gemini-2.5-flash-lite",
            vec![Content::text("Return JSON")],
        )
        .await
        .unwrap();
    assert_eq!(response.answer, "ok");
}

#[tokio::test]
async fn mock_json_generation_rejects_non_json_mime() {
    let client = Client::new("test-key").unwrap();
    let err = client
        .models()
        .generate_json_with_config::<JsonSmokeResponse>(
            "gemini-2.5-flash-lite",
            vec![Content::text("Return JSON")],
            GenerateContentConfig {
                generation_config: Some(GenerationConfig {
                    response_mime_type: Some("text/plain".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();

    match err {
        rust_genai::Error::InvalidConfig { message } => {
            assert!(message.contains("response_mime_type = application/json"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn mock_json_generation_rejects_invalid_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.5-flash-lite:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "not-json"}
                    ]
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();

    let err = client
        .models()
        .generate_json::<JsonSmokeResponse>(
            "gemini-2.5-flash-lite",
            vec![Content::text("Return JSON")],
        )
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::Serialization { .. }));
}
