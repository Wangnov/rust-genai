use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::Client;

#[tokio::test]
async fn api_key_header_is_inserted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1beta/models"))
        .and(header("x-goog-api-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "models": []
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();
    let models = client.models();
    let _ = models.list().await.unwrap();
}

#[tokio::test]
async fn api_key_header_respects_custom_value() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1beta/models"))
        .and(header("x-goog-api-key", "custom-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "models": []
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .header("x-goog-api-key", "custom-key")
        .base_url(server.uri())
        .build()
        .unwrap();
    let models = client.models();
    let _ = models.list().await.unwrap();
}
