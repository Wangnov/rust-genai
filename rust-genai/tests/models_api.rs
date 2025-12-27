use futures_util::StreamExt;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::content::Content;
use rust_genai::types::models::ListModelsConfig;
use rust_genai::Client;

#[tokio::test]
async fn test_generate_content_gemini_api() {
    let mock_server = MockServer::start().await;
    let response_body = json!({
        "candidates": [
            {
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "Hello"}
                    ]
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let response = client
        .models()
        .generate_content("gemini-2.0-flash", vec![Content::text("Test")])
        .await
        .unwrap();
    assert_eq!(response.text(), Some("Hello".to_string()));
}

#[tokio::test]
async fn test_sse_streaming() {
    let mock_server = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.0-flash:streamGenerateContent",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let mut stream = client
        .models()
        .generate_content_stream(
            "gemini-2.0-flash",
            vec![Content::text("Test")],
            Default::default(),
        )
        .await
        .unwrap();

    let mut texts = Vec::new();
    while let Some(item) = stream.next().await {
        let response = item.unwrap();
        if let Some(text) = response.text() {
            texts.push(text);
        }
    }

    assert_eq!(texts, vec!["Hi".to_string()]);
}

#[tokio::test]
async fn test_list_models_with_query_params() {
    let mock_server = MockServer::start().await;
    let response_body = json!({
        "models": []
    });

    Mock::given(method("GET"))
        .and(path("/v1beta/models"))
        .and(query_param("pageSize", "5"))
        .and(query_param("pageToken", "token-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(mock_server.uri())
        .build()
        .unwrap();

    let _ = client
        .models()
        .list_with_config(ListModelsConfig {
            page_size: Some(5),
            page_token: Some("token-1".to_string()),
            filter: None,
            query_base: None,
        })
        .await
        .unwrap();
}
