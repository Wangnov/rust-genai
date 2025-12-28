use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::batches::{
    BatchJobSource, CreateBatchJobConfig, InlinedRequest, ListBatchJobsConfig,
};
use rust_genai::types::content::Content;

mod support;
use support::build_gemini_client_with_version;

#[tokio::test]
async fn batches_error_responses() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:batchGenerateContent"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/batches/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/batches"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1beta/batches/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let batches = client.batches();

    let src = BatchJobSource {
        inlined_requests: Some(vec![InlinedRequest {
            model: Some("gemini-1.5-pro".to_string()),
            contents: Some(vec![Content::text("hello")]),
            metadata: None,
            config: None,
        }]),
        ..Default::default()
    };

    let err = batches
        .create("gemini-1.5-pro", src, CreateBatchJobConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = batches.get("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = batches.list().await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = batches.delete("bad").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));
}

#[tokio::test]
async fn batches_api_flow() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:batchGenerateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "batches/1",
            "metadata": {
                "displayName": "job-1",
                "state": "ACTIVE",
                "model": "models/gemini-1.5-pro"
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/batches/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "batches/1",
            "metadata": {"state": "ACTIVE"}
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/batches/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing"))
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1beta/batches/1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/batches"))
        .and(query_param("pageSize", "2"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "operations": [{"name": "batches/1"}],
            "nextPageToken": "next"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/batches"))
        .and(query_param("pageSize", "2"))
        .and(query_param("pageToken", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "operations": [{"name": "batches/2"}]
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let batches = client.batches();

    let created = batches
        .create(
            "gemini-1.5-pro",
            BatchJobSource {
                file_name: Some("file.jsonl".to_string()),
                ..Default::default()
            },
            CreateBatchJobConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(created.name.as_deref(), Some("batches/1"));

    let job = batches.get("1").await.unwrap();
    assert_eq!(job.name.as_deref(), Some("batches/1"));

    let err = batches.get("missing").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    batches.delete("1").await.unwrap();

    let list = batches
        .list_with_config(ListBatchJobsConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(list.batch_jobs.unwrap().len(), 1);

    let all = batches
        .all_with_config(ListBatchJobsConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}
