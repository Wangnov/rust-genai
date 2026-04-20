use http::Method;
use serde_json::json;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use rust_genai::types::http::HttpRetryOptions;
use rust_genai::{Backend, Client, Credentials};

pub struct GeminiTestContext {
    pub client: Client,
    _server: MockServer,
}

pub struct VertexTestContext {
    pub client: Client,
    _server: MockServer,
    _temp_dir: tempfile::TempDir,
}

fn disabled_retry_options() -> HttpRetryOptions {
    HttpRetryOptions {
        attempts: Some(1),
        ..Default::default()
    }
}

pub async fn setup_mock_gemini_context() -> GeminiTestContext {
    let server = MockServer::start().await;
    mount_default_mock(&server).await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .retry_options(disabled_retry_options())
        .build()
        .unwrap();

    GeminiTestContext {
        client,
        _server: server,
    }
}

pub async fn setup_mock_vertex_context() -> VertexTestContext {
    let server = MockServer::start().await;
    mount_default_mock(&server).await;

    let dir = tempfile::tempdir().unwrap();
    let client_secret_path = dir.path().join("client_secret.json");
    let token_cache_path = dir.path().join("token.json");
    let token_uri = format!("{}/token", server.uri());
    std::fs::write(
        &client_secret_path,
        json!({
            "installed": {
                "client_id": "client-id",
                "client_secret": "client-secret",
                "token_uri": token_uri
            }
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        &token_cache_path,
        json!({
            "refresh_token": "refresh-1",
            "token_uri": format!("{}/token", server.uri())
        })
        .to_string(),
    )
    .unwrap();

    let client = Client::builder()
        .backend(Backend::VertexAi)
        .vertex_project("proj")
        .vertex_location("us-central1")
        .base_url(server.uri())
        .retry_options(disabled_retry_options())
        .credentials(Credentials::OAuth {
            client_secret_path,
            token_cache_path: Some(token_cache_path),
        })
        .build()
        .unwrap();

    VertexTestContext {
        client,
        _server: server,
        _temp_dir: dir,
    }
}

pub fn build_live_gemini_client() -> rust_genai::Result<Client> {
    let api_key = first_nonempty_env(&["GEMINI_API_KEY", "GOOGLE_API_KEY"]).ok_or_else(|| {
        rust_genai::Error::InvalidConfig {
            message: "GEMINI_API_KEY or GOOGLE_API_KEY not found".into(),
        }
    })?;

    Client::builder().api_key(api_key).build()
}

pub fn build_live_vertex_client() -> rust_genai::Result<Client> {
    let project = required_env("GOOGLE_CLOUD_PROJECT")
        .map_err(|message| rust_genai::Error::InvalidConfig { message })?;
    let location = required_env("GOOGLE_CLOUD_LOCATION")
        .map_err(|message| rust_genai::Error::InvalidConfig { message })?;

    Client::builder()
        .backend(Backend::VertexAi)
        .vertex_project(project)
        .vertex_location(location)
        .credentials(Credentials::ApplicationDefault)
        .build()
}

pub fn live_gemini_model() -> String {
    env_or_default("GENAI_CONFORMANCE_GEMINI_MODEL", "gemini-2.5-flash-lite")
}

pub fn live_vertex_model() -> String {
    env_or_default(
        "GENAI_CONFORMANCE_VERTEX_MODEL",
        "publishers/google/models/gemini-2.5-flash-lite",
    )
}

pub fn expensive_opt_in() -> bool {
    env_flag("GENAI_CONFORMANCE_ENABLE_EXPENSIVE")
        || env_flag("GENAI_CONFORMANCE_ENABLE_FILE_UPLOAD")
}

fn required_env(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} not found"))
}

fn first_nonempty_env(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn env_or_default(name: &str, default: &str) -> String {
    first_nonempty_env(&[name]).unwrap_or_else(|| default.to_string())
}

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
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
        "data: {\"event_type\":\"interaction.start\",\"event_id\":\"evt_1\",\"interaction\":{\"id\":\"int_1\",\"status\":\"in_progress\"}}\n\n",
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

            if path.contains(":streamGenerateContent") {
                return ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(stream_body.clone());
            }

            let accepts_sse = req
                .headers
                .get("accept")
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.contains("text/event-stream"));
            if accepts_sse && path.ends_with("/interactions") {
                return ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(interaction_stream_body.clone());
            }
            if accepts_sse && path.contains("/interactions/") {
                return ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(interaction_stream_body.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce()) {
        let _guard = ENV_LOCK.lock().unwrap();
        let backup: Vec<(String, Option<String>)> = vars
            .iter()
            .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
            .collect();

        for (key, value) in vars {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }

        f();

        for (key, value) in backup {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    fn live_gemini_model_uses_default_for_blank_override() {
        with_env(&[("GENAI_CONFORMANCE_GEMINI_MODEL", Some("   "))], || {
            assert_eq!(live_gemini_model(), "gemini-2.5-flash-lite");
        });
    }

    #[test]
    fn live_vertex_model_uses_default_for_blank_override() {
        with_env(&[("GENAI_CONFORMANCE_VERTEX_MODEL", Some(""))], || {
            assert_eq!(
                live_vertex_model(),
                "publishers/google/models/gemini-2.5-flash-lite"
            );
        });
    }

    #[test]
    fn live_model_helpers_preserve_trimmed_override() {
        with_env(
            &[
                ("GENAI_CONFORMANCE_GEMINI_MODEL", Some(" gemini-custom ")),
                (
                    "GENAI_CONFORMANCE_VERTEX_MODEL",
                    Some(" publishers/google/models/gemini-custom "),
                ),
            ],
            || {
                assert_eq!(live_gemini_model(), "gemini-custom");
                assert_eq!(
                    live_vertex_model(),
                    "publishers/google/models/gemini-custom"
                );
            },
        );
    }
}
