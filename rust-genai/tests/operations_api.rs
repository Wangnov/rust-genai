use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::operations::ListOperationsConfig;

mod support;
use support::build_gemini_client_with_version;

#[tokio::test]
async fn operations_api_flow_and_errors() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1beta/operations/op1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "operations/op1",
            "done": true
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/operations/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/operations"))
        .and(query_param("pageSize", "2"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "operations": [{"name": "operations/op1"}],
            "nextPageToken": "next"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/operations"))
        .and(query_param("pageSize", "2"))
        .and(query_param("pageToken", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "operations": [{"name": "operations/op2"}]
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let ops = client.operations();

    let op = ops.get("op1").await.unwrap();
    assert_eq!(op.name.as_deref(), Some("operations/op1"));

    let err = ops.get("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let list = ops
        .list_with_config(ListOperationsConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(list.operations.unwrap().len(), 1);

    let all = ops
        .all_with_config(ListOperationsConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}
