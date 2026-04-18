use futures_util::StreamExt;
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::interactions::{CreateInteractionConfig, WebhookConfig};

mod support;
use support::build_gemini_client_with_version;

#[tokio::test]
async fn interactions_api_flow() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/interactions"))
        .and(body_json(
            json!({"model": "gemini-3-flash-preview", "input": "hi"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "int_1",
            "model": "gemini-3-flash-preview"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/interactions"))
        .and(body_json(json!({"model": "gemini-3-flash-preview", "input": "hi", "stream": true})))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(concat!(
                    "data: {\"event_type\":\"interaction.start\",\"event_id\":\"evt_1\",\"interaction\":{\"id\":\"int_1\",\"status\":\"in_progress\"}}\n\n",
                    "data: [DONE]\n\n"
                )),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/interactions/int_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "int_1",
            "model": "gemini-3-flash-preview"
        })))
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1beta/interactions/int_1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/interactions/int_1/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "int_1",
            "model": "gemini-3-flash-preview"
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let interactions = client.interactions();

    let created = interactions
        .create(CreateInteractionConfig::new("gemini-3-flash-preview", "hi"))
        .await
        .unwrap();
    assert_eq!(created.id.as_deref(), Some("int_1"));

    let mut stream = interactions
        .create_stream(CreateInteractionConfig::new("gemini-3-flash-preview", "hi"))
        .await
        .unwrap();
    let mut saw_event = false;
    while let Some(item) = stream.next().await {
        let event = item.unwrap();
        if event.event_type.as_deref() == Some("interaction.start") {
            saw_event = true;
        }
    }
    assert!(saw_event);

    let got = interactions.get("int_1").await.unwrap();
    assert_eq!(got.id.as_deref(), Some("int_1"));

    let cancelled = interactions.cancel("int_1").await.unwrap();
    assert_eq!(cancelled.id.as_deref(), Some("int_1"));

    interactions.delete("int_1").await.unwrap();
}

#[tokio::test]
async fn interactions_error_responses_and_empty_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/interactions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/interactions/int_empty"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/interactions/int_bad/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not-json"))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let interactions = client.interactions();

    let err = interactions
        .create(CreateInteractionConfig::new("gemini-3-flash-preview", "hi"))
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let empty = interactions.get("int_empty").await.unwrap();
    assert!(empty.id.is_none());

    let err = interactions.cancel("int_bad").await.unwrap_err();
    assert!(matches!(
        err,
        rust_genai::Error::Serialization { .. } | rust_genai::Error::Parse { .. }
    ));
}

#[tokio::test]
async fn interactions_create_supports_webhook_config() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/interactions"))
        .and(body_json(json!({
            "model": "gemini-3-flash-preview",
            "input": "hi",
            "webhook_config": {
                "uris": ["https://example.com/webhook"],
                "user_metadata": {"job_id": "int_1"}
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "int_1",
            "model": "gemini-3-flash-preview"
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let created = client
        .interactions()
        .create(CreateInteractionConfig {
            webhook_config: Some(WebhookConfig {
                uris: Some(vec!["https://example.com/webhook".to_string()]),
                user_metadata: Some([("job_id".to_string(), json!("int_1"))].into()),
            }),
            ..CreateInteractionConfig::new("gemini-3-flash-preview", "hi")
        })
        .await
        .unwrap();
    assert_eq!(created.id.as_deref(), Some("int_1"));
}
