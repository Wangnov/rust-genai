use serde_json::json;
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::http::HttpOptions;
use rust_genai::types::tunings::{
    CreateTuningJobConfig, ListTuningJobsConfig, TuningDataset, TuningExample,
};

mod support;
use support::build_gemini_client_with_version;

#[tokio::test]
async fn tuning_api_error_responses() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/tunedModels"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/tunedModels/1"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1beta/tunedModels"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1beta/tunedModels/1:cancel"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let tunings = client.tunings();
    let err = tunings
        .tune("gemini-1.5-pro", TuningDataset::default())
        .await
        .unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = tunings.get("tunedModels/1").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = tunings.list().await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));

    let err = tunings.cancel("tunedModels/1").await.unwrap_err();
    assert!(matches!(err, rust_genai::Error::ApiError { .. }));
}

#[tokio::test]
async fn tunings_api_flow() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/tunedModels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "tunedModels/1",
            "state": "ACTIVE",
            "baseModel": "models/gemini-1.5-pro"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/tunedModels/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "tunedModels/1",
            "state": "ACTIVE"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/tunedModels/1:cancel"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/tunedModels"))
        .and(query_param("pageSize", "2"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tunedModels": [{"name": "tunedModels/1"}],
            "nextPageToken": "next"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/tunedModels"))
        .and(query_param("pageSize", "2"))
        .and(query_param("pageToken", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tunedModels": [{"name": "tunedModels/2"}]
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let tunings = client.tunings();

    let dataset = TuningDataset {
        examples: Some(vec![TuningExample {
            text_input: Some("hi".to_string()),
            output: Some("ok".to_string()),
        }]),
        ..Default::default()
    };
    let created = tunings.tune("gemini-1.5-pro", dataset).await.unwrap();
    assert_eq!(created.name.as_deref(), Some("tunedModels/1"));

    let got = tunings.get("1").await.unwrap();
    assert_eq!(got.name.as_deref(), Some("tunedModels/1"));

    tunings.cancel("1").await.unwrap();

    let list = tunings
        .list_with_config(ListTuningJobsConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(list.tuning_jobs.unwrap().len(), 1);

    let all = tunings
        .all_with_config(ListTuningJobsConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn tune_with_extra_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/tunedModels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "tunedModels/extra"
        })))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let tunings = client.tunings();
    let dataset = TuningDataset {
        examples: Some(vec![TuningExample {
            text_input: Some("hi".to_string()),
            output: Some("ok".to_string()),
        }]),
        ..Default::default()
    };
    let config = CreateTuningJobConfig {
        http_options: Some(HttpOptions {
            extra_body: Some(json!({"extra": "x"})),
            ..Default::default()
        }),
        ..Default::default()
    };

    let job = tunings
        .tune_with_config("gemini-1.5-pro", dataset, config)
        .await
        .unwrap();
    assert_eq!(job.name.as_deref(), Some("tunedModels/extra"));
}
