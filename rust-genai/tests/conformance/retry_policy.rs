use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use rust_genai::types::caches::ListCachedContentsConfig;
use rust_genai::types::http::{HttpOptions, HttpRetryOptions};
use rust_genai::Client;

#[derive(Clone)]
struct SequenceResponder {
    calls: Arc<AtomicUsize>,
    first: ResponseTemplate,
    second: ResponseTemplate,
}

impl Respond for SequenceResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let idx = self.calls.fetch_add(1, Ordering::SeqCst);
        if idx == 0 {
            self.first.clone()
        } else {
            self.second.clone()
        }
    }
}

fn no_delay_retry_options(attempts: u32, codes: Vec<u16>) -> HttpRetryOptions {
    HttpRetryOptions {
        attempts: Some(attempts),
        initial_delay: Some(0.0),
        max_delay: Some(0.0),
        exp_base: Some(0.0),
        jitter: Some(0.0),
        http_status_codes: Some(codes),
    }
}

#[tokio::test]
async fn mock_retry_policy_global_retries_and_succeeds() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(SequenceResponder {
            calls: calls.clone(),
            first: ResponseTemplate::new(500).set_body_string("oops"),
            second: ResponseTemplate::new(200).set_body_json(json!({
                "cachedContents": []
            })),
        })
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .api_version("v1beta")
        .retry_options(no_delay_retry_options(2, vec![500]))
        .build()
        .unwrap();

    let resp = client.caches().list().await.unwrap();
    assert_eq!(resp.cached_contents.unwrap_or_default().len(), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn mock_retry_policy_defaults_retry_transient_errors() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(SequenceResponder {
            calls: calls.clone(),
            first: ResponseTemplate::new(500)
                .insert_header("retry-after", "0")
                .set_body_string("oops"),
            second: ResponseTemplate::new(500)
                .insert_header("retry-after", "0")
                .set_body_string("oops"),
        })
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .api_version("v1beta")
        .build()
        .unwrap();

    let err = client.caches().list().await.unwrap_err();
    assert_eq!(err.status().unwrap().as_u16(), 500);
    assert_eq!(err.attempts(), Some(5));
    assert!(err.is_retryable());
    assert_eq!(calls.load(Ordering::SeqCst), 5);
}

#[tokio::test]
async fn mock_retry_policy_per_request_override() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(SequenceResponder {
            calls: calls.clone(),
            first: ResponseTemplate::new(500).set_body_string("oops"),
            second: ResponseTemplate::new(200).set_body_json(json!({
                "cachedContents": []
            })),
        })
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .api_version("v1beta")
        .retry_options(no_delay_retry_options(1, vec![429]))
        .build()
        .unwrap();

    let resp = client
        .caches()
        .list_with_config(ListCachedContentsConfig {
            http_options: Some(HttpOptions {
                retry_options: Some(no_delay_retry_options(2, vec![500])),
                ..Default::default()
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(resp.cached_contents.unwrap_or_default().len(), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn mock_retry_policy_non_retryable_status() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(SequenceResponder {
            calls: calls.clone(),
            first: ResponseTemplate::new(400).set_body_string("bad"),
            second: ResponseTemplate::new(200).set_body_json(json!({
                "cachedContents": []
            })),
        })
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .api_version("v1beta")
        .retry_options(no_delay_retry_options(2, vec![500]))
        .build()
        .unwrap();

    let err = client.caches().list().await.unwrap_err();
    assert!(matches!(
        err,
        rust_genai::Error::ApiError { status: 400, .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mock_retry_policy_attempts_one_disables_retry() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(SequenceResponder {
            calls: calls.clone(),
            first: ResponseTemplate::new(500).set_body_string("oops"),
            second: ResponseTemplate::new(200).set_body_json(json!({
                "cachedContents": []
            })),
        })
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .api_version("v1beta")
        .retry_options(no_delay_retry_options(1, vec![500]))
        .build()
        .unwrap();

    let err = client.caches().list().await.unwrap_err();
    assert!(matches!(
        err,
        rust_genai::Error::ApiError { status: 500, .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mock_retry_policy_attempts_one_preserves_custom_retryability() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "error": {
                "message": "conflict",
                "status": "ABORTED"
            }
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .api_version("v1beta")
        .retry_options(no_delay_retry_options(1, vec![409]))
        .build()
        .unwrap();

    let err = client.caches().list().await.unwrap_err();
    assert_eq!(err.status().unwrap().as_u16(), 409);
    assert_eq!(err.attempts(), Some(1));
    assert!(err.is_retryable());
}

#[tokio::test]
async fn mock_retry_policy_error_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "3")
                .set_body_json(json!({
                    "error": {
                        "message": "slow down",
                        "status": "RESOURCE_EXHAUSTED",
                        "details": [{"quota": "tokens"}]
                    }
                })),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .api_version("v1beta")
        .retry_options(no_delay_retry_options(1, vec![429]))
        .build()
        .unwrap();

    let err = client.caches().list().await.unwrap_err();
    assert_eq!(err.status().unwrap().as_u16(), 429);
    assert_eq!(err.code().as_deref(), Some("RESOURCE_EXHAUSTED"));
    assert_eq!(err.retry_after().map(|delay| delay.as_secs()), Some(3));
    assert_eq!(err.attempts(), Some(1));
    assert!(err.is_rate_limited());
}

#[tokio::test]
async fn mock_retry_policy_tracks_attempt_count_after_retries() {
    let server = MockServer::start().await;
    let calls = Arc::new(AtomicUsize::new(0));
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(SequenceResponder {
            calls: calls.clone(),
            first: ResponseTemplate::new(503).set_body_json(json!({
                "error": {"message": "busy", "status": "UNAVAILABLE"}
            })),
            second: ResponseTemplate::new(503).set_body_json(json!({
                "error": {"message": "still busy", "status": "UNAVAILABLE"}
            })),
        })
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .api_version("v1beta")
        .retry_options(no_delay_retry_options(2, vec![503]))
        .build()
        .unwrap();

    let err = client.caches().list().await.unwrap_err();
    assert_eq!(err.status().unwrap().as_u16(), 503);
    assert_eq!(err.attempts(), Some(2));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
