use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::files::WaitForFileConfig;
use rust_genai::types::enums::FileState;
use rust_genai::types::files::{DownloadFileConfig, ListFilesConfig};

mod support;
use support::build_gemini_client;

#[tokio::test]
async fn files_api_flow() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1beta/files/file1:download"))
        .and(query_param("alt", "media"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1u8, 2, 3]))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "files": [{"name": "files/file1"}],
            "nextPageToken": "next"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param("pageToken", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "files": [{"name": "files/file2"}]
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/files/file1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "files/file1",
            "state": "ACTIVE"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/files/failed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "files/failed",
            "state": "FAILED"
        })))
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1beta/files/file1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let files = client.files();

    let bytes = files.download("file1").await.unwrap();
    assert_eq!(bytes, vec![1u8, 2, 3]);

    let list = files
        .list_with_config(ListFilesConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(list.files.unwrap().len(), 1);

    let all = files
        .all_with_config(ListFilesConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.len(), 2);

    let file = files.get("file1").await.unwrap();
    assert_eq!(file.name.as_deref(), Some("files/file1"));

    files.delete("file1").await.unwrap();

    let active = files
        .wait_for_active("file1", WaitForFileConfig::default())
        .await
        .unwrap();
    assert_eq!(active.state, Some(FileState::Active));

    let err = files
        .wait_for_active(
            "failed",
            WaitForFileConfig {
                poll_interval: std::time::Duration::from_millis(1),
                timeout: Some(std::time::Duration::from_millis(1)),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        rust_genai::Error::ApiError { .. } | rust_genai::Error::Timeout { .. }
    ));
}

#[tokio::test]
async fn files_error_responses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1beta/files/bad:download"))
        .and(query_param("alt", "media"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/files"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/files/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1beta/files/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let files = client.files();

    let err = files.download("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = files.list().await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = files.get("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = files.delete("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));
}

#[tokio::test]
async fn download_with_config() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1beta/files/file1:download"))
        .and(query_param("alt", "media"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![9u8, 8]))
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let files = client.files();

    let bytes = files
        .download_with_config("file1", DownloadFileConfig::default())
        .await
        .unwrap();
    assert_eq!(bytes, vec![9u8, 8]);
}

#[tokio::test]
async fn wait_for_active_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1beta/files/pending"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "files/pending",
            "state": "PROCESSING"
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let files = client.files();

    let err = files
        .wait_for_active(
            "pending",
            WaitForFileConfig {
                poll_interval: std::time::Duration::from_millis(1),
                timeout: Some(std::time::Duration::from_millis(0)),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::Timeout { .. }));
}
