use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use rust_genai::types::http::HttpOptions;
use rust_genai::types::tokens::CreateAuthTokenConfig;

mod support;
use support::build_gemini_client;

#[derive(Clone)]
struct AuthTokenResponder {
    calls: Arc<AtomicUsize>,
}

impl Respond for AuthTokenResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let idx = self.calls.fetch_add(1, Ordering::SeqCst);
        if idx == 0 {
            ResponseTemplate::new(200).set_body_json(json!({
                "name": "auth_tokens/1"
            }))
        } else {
            ResponseTemplate::new(400).set_body_string("bad")
        }
    }
}

#[tokio::test]
async fn auth_tokens_create_success_and_error() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST"))
        .and(path("/v1beta/auth_tokens"))
        .respond_with(AuthTokenResponder { calls })
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let tokens = client.auth_tokens();

    let token = tokens
        .create(CreateAuthTokenConfig {
            uses: Some(1),
            http_options: Some(HttpOptions {
                extra_body: Some(json!({"extra": "value"})),
                ..Default::default()
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(token.name.as_deref(), Some("auth_tokens/1"));

    let err = tokens
        .create(CreateAuthTokenConfig {
            uses: Some(1),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let received = server.received_requests().await.unwrap();
    let body = String::from_utf8_lossy(&received[0].body);
    assert!(body.contains(r#""extra":"value""#));
}
