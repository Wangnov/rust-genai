use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::documents::{DeleteDocumentConfig, ListDocumentsConfig};

mod support;
use support::build_gemini_client_with_version;

#[tokio::test]
async fn documents_api_flow() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores/store/documents/doc1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "fileSearchStores/store/documents/doc1"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores/store/documents/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing"))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1beta/fileSearchStores/store/documents/doc1"))
        .and(query_param("force", "true"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores/store/documents"))
        .and(query_param("pageSize", "2"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "documents": [{"name": "fileSearchStores/store/documents/doc1"}],
            "nextPageToken": "next"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores/store/documents"))
        .and(query_param("pageSize", "2"))
        .and(query_param("pageToken", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "documents": [{"name": "fileSearchStores/store/documents/doc2"}]
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let docs = client.documents();

    let doc = docs
        .get("fileSearchStores/store/documents/doc1")
        .await
        .unwrap();
    assert_eq!(
        doc.name.as_deref(),
        Some("fileSearchStores/store/documents/doc1")
    );

    let err = docs
        .get("fileSearchStores/store/documents/missing")
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    docs.delete_with_config(
        "fileSearchStores/store/documents/doc1",
        DeleteDocumentConfig {
            force: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let list = docs
        .list_with_config(
            "store",
            ListDocumentsConfig {
                page_size: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(list.documents.unwrap().len(), 1);

    let all = docs
        .all_with_config(
            "store",
            ListDocumentsConfig {
                page_size: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn documents_list_error_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores/store/documents"))
        .respond_with(ResponseTemplate::new(500).set_body_string("bad"))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let docs = client.documents();
    let err = docs.list("store").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));
}

#[tokio::test]
async fn documents_delete_error_response() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/v1beta/fileSearchStores/store/documents/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let docs = client.documents();
    let err = docs
        .delete("fileSearchStores/store/documents/bad")
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));
}
