#![allow(dead_code)]

use http::Method;
use serde_json::json;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use rust_genai::Client;

pub fn build_gemini_client(base_url: &str) -> Client {
    Client::builder()
        .api_key("test-key")
        .base_url(base_url)
        .build()
        .unwrap()
}

pub fn build_gemini_client_with_version(base_url: &str, api_version: &str) -> Client {
    Client::builder()
        .api_key("test-key")
        .base_url(base_url)
        .api_version(api_version)
        .build()
        .unwrap()
}

pub async fn mount_default_mock(server: &MockServer) {
    let server_uri = server.uri();
    let upload_url = format!("{server_uri}/upload-session");
    let stream_body = concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]}}]}\n\n",
        "data: [DONE]\n\n"
    )
    .to_string();
    let interaction_stream_body = concat!(
        "data: {\"event_type\":\"interactions.create\",\"data\":{\"id\":\"int_1\",\"model\":\"gemini-2.0-flash\"}}\n\n",
        "data: [DONE]\n\n"
    )
    .to_string();

    Mock::given(any())
        .respond_with(move |req: &Request| {
            let path = req.url.path();
            if let Some(command) = req
                .headers
                .get("x-goog-upload-command")
                .and_then(|value| value.to_str().ok())
            {
                if command == "start" {
                    return ResponseTemplate::new(200)
                        .insert_header("x-goog-upload-url", upload_url.clone());
                }
            }

            if path == "/upload-session" {
                let command = req
                    .headers
                    .get("x-goog-upload-command")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("");
                let status = if command.contains("finalize") {
                    "final"
                } else {
                    "active"
                };
                let mut response =
                    ResponseTemplate::new(200).insert_header("x-goog-upload-status", status);
                if status == "final" {
                    response = response.set_body_json(json!({"name": "files/abc"}));
                }
                return response;
            }

            if req.url.query().is_some_and(|q| q.contains("alt=sse")) {
                return ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(interaction_stream_body.clone());
            }

            if path.contains(":streamGenerateContent") {
                return ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(stream_body.clone());
            }

            if path.contains(":generateContent") {
                return ResponseTemplate::new(200).set_body_json(json!({
                    "candidates": [
                        {"content": {"role": "model", "parts": [{"text": "ok"}]}}
                    ]
                }));
            }

            if path.contains("/files/") && req.method == Method::GET && !path.contains(":download")
            {
                return ResponseTemplate::new(200).set_body_json(json!({
                    "name": "files/abc",
                    "state": "ACTIVE"
                }));
            }

            if path == "/token" {
                return ResponseTemplate::new(200).set_body_json(json!({
                    "access_token": "token-1",
                    "expires_in": 3600
                }));
            }

            ResponseTemplate::new(200).set_body_json(json!({
                "name": "resource-1",
                "models": [],
                "files": [],
                "cachedContents": [],
                "batchPredictionJobs": [],
                "operations": [],
                "fileSearchStores": [],
                "documents": [],
                "tuningJobs": [],
                "nextPageToken": ""
            }))
        })
        .mount(server)
        .await;
}
