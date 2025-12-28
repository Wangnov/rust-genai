use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::caches::{
    CreateCachedContentConfig, ListCachedContentsConfig, UpdateCachedContentConfig,
};

mod support;
use support::build_gemini_client_with_version;

#[tokio::test]
async fn caches_api_flow() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "cachedContents/1"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "cachedContents/1"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing"))
        .mount(&server)
        .await;

    Mock::given(method("PATCH"))
        .and(path("/v1beta/cachedContents/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "cachedContents/1",
            "expireTime": "2024-01-01T00:00:00Z"
        })))
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1beta/cachedContents/1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .and(query_param("pageSize", "2"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "cachedContents": [{"name": "cachedContents/1"}],
            "nextPageToken": "next"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .and(query_param("pageSize", "2"))
        .and(query_param("pageToken", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "cachedContents": [{"name": "cachedContents/2"}]
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let caches = client.caches();

    let created = caches
        .create("gemini-1.5-pro", CreateCachedContentConfig::default())
        .await
        .unwrap();
    assert_eq!(created.name.as_deref(), Some("cachedContents/1"));

    let got = caches.get("1").await.unwrap();
    assert_eq!(got.name.as_deref(), Some("cachedContents/1"));

    let err = caches.get("missing").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let updated = caches
        .update("1", UpdateCachedContentConfig::default())
        .await
        .unwrap();
    assert_eq!(updated.name.as_deref(), Some("cachedContents/1"));

    caches.delete("1").await.unwrap();

    let list = caches
        .list_with_config(ListCachedContentsConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(list.cached_contents.unwrap().len(), 1);

    let all = caches
        .all_with_config(ListCachedContentsConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn caches_error_responses() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/cachedContents"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1beta/cachedContents/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1beta/cachedContents/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let caches = client.caches();

    let err = caches
        .create("gemini-1.5-pro", CreateCachedContentConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = caches.get("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = caches.list().await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = caches
        .update("bad", UpdateCachedContentConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = caches.delete("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));
}
