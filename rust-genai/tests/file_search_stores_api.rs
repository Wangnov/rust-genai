use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::file_search_stores::{
    CreateFileSearchStoreConfig, DeleteFileSearchStoreConfig, ImportFileConfig,
    ListFileSearchStoresConfig, UploadToFileSearchStoreConfig,
};
use rust_genai::types::http::HttpOptions;

mod support;
use support::build_gemini_client_with_version;

#[tokio::test]
async fn file_search_stores_api_flow() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/fileSearchStores"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "fileSearchStores/store1"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores/store1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "fileSearchStores/store1"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing"))
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1beta/fileSearchStores/store1"))
        .and(query_param("force", "true"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores"))
        .and(query_param("pageSize", "2"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "fileSearchStores": [{"name": "fileSearchStores/store1"}],
            "nextPageToken": "next"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores"))
        .and(query_param("pageSize", "2"))
        .and(query_param("pageToken", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "fileSearchStores": [{"name": "fileSearchStores/store2"}]
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let stores = client.file_search_stores();

    let created = stores
        .create(CreateFileSearchStoreConfig::default())
        .await
        .unwrap();
    assert_eq!(created.name.as_deref(), Some("fileSearchStores/store1"));

    let got = stores.get("store1").await.unwrap();
    assert_eq!(got.name.as_deref(), Some("fileSearchStores/store1"));

    let err = stores.get("missing").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    stores
        .delete_with_config(
            "store1",
            DeleteFileSearchStoreConfig {
                force: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let list = stores
        .list_with_config(ListFileSearchStoresConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(list.file_search_stores.unwrap().len(), 1);

    let all = stores
        .all_with_config(ListFileSearchStoresConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn file_search_store_error_responses() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/fileSearchStores"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/fileSearchStores"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1beta/fileSearchStores/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let stores = client.file_search_stores();

    let err = stores
        .create(CreateFileSearchStoreConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = stores.get("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = stores.list().await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = stores.delete("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));
}

#[tokio::test]
async fn upload_from_path_with_mime_type_and_import_file() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/upload/v1beta/fileSearchStores/store:uploadToFileSearchStore",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-goog-upload-url", format!("{}/upload-ok", server.uri())),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/upload-ok"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-goog-upload-status", "final")
                .set_body_json(json!({"name": "operations/ok"})),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/fileSearchStores/store:importFile"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "operations/import"
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let stores = client.file_search_stores();

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("file.txt");
    tokio::fs::write(&file_path, b"hello").await.unwrap();
    let config = UploadToFileSearchStoreConfig {
        mime_type: Some("text/plain".to_string()),
        http_options: Some(HttpOptions {
            base_url: Some(format!("{}/", server.uri())),
            api_version: Some("v1beta".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let op = stores
        .upload_to_file_search_store_from_path("fileSearchStores/store", &file_path, config)
        .await
        .unwrap();
    assert_eq!(op.name.as_deref(), Some("operations/ok"));

    let op = stores
        .import_file(
            "fileSearchStores/store",
            "files/123",
            ImportFileConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(op.name.as_deref(), Some("operations/import"));
}
