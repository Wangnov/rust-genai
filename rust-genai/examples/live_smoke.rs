use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use rust_genai::files::WaitForFileConfig;
use rust_genai::types::content::Content;
use rust_genai::types::models::{GenerateContentConfig, Model};
use rust_genai::Client;

#[derive(Debug)]
struct StepResult {
    name: &'static str,
    ok: bool,
    detail: String,
}

impl StepResult {
    fn pass(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            ok: true,
            detail: detail.into(),
        }
    }

    fn fail(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            ok: false,
            detail: detail.into(),
        }
    }
}

fn bare_model_name(name: &str) -> &str {
    name.strip_prefix("models/").unwrap_or(name)
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

fn supports_action(model: &Model, action: &str) -> bool {
    model
        .supported_actions
        .as_ref()
        .is_some_and(|actions| actions.iter().any(|item| item.eq_ignore_ascii_case(action)))
}

fn supports_action_or_unknown(model: &Model, action: &str) -> bool {
    model
        .supported_actions
        .as_ref()
        .map(|actions| {
            actions.is_empty() || actions.iter().any(|item| item.eq_ignore_ascii_case(action))
        })
        .unwrap_or(true)
}

fn choose_generation_model(models: &[Model]) -> Option<String> {
    let mut candidates: Vec<String> = models
        .iter()
        .filter_map(|model| {
            let name = bare_model_name(model.name.as_deref()?).to_string();
            let lower = name.to_ascii_lowercase();
            if !supports_action_or_unknown(model, "generateContent") {
                return None;
            }
            if ["embedding", "image", "tts", "live", "veo", "imagen"]
                .iter()
                .any(|part| lower.contains(part))
            {
                return None;
            }
            Some(name)
        })
        .collect();
    candidates.sort();

    for fragment in [
        "gemini-2.5-flash-lite",
        "gemini-flash-lite-latest",
        "gemini-2.0-flash-lite",
        "flash-lite",
        "gemini-2.5-flash",
        "gemini-2.0-flash",
        "flash",
    ] {
        if let Some(name) = candidates
            .iter()
            .find(|candidate| candidate.to_ascii_lowercase().contains(fragment))
        {
            return Some(name.clone());
        }
    }

    candidates.into_iter().next()
}

fn choose_embedding_model(models: &[Model]) -> Option<String> {
    let mut candidates: Vec<String> = models
        .iter()
        .filter_map(|model| {
            let name = bare_model_name(model.name.as_deref()?).to_string();
            if supports_action(model, "embedContent")
                || name.to_ascii_lowercase().contains("embedding")
            {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    candidates.sort();

    for fragment in ["gemini-embedding-001", "gemini-embedding-2", "embedding"] {
        if let Some(name) = candidates
            .iter()
            .find(|candidate| candidate.to_ascii_lowercase().contains(fragment))
        {
            return Some(name.clone());
        }
    }

    candidates.into_iter().next()
}

fn shorten(value: impl AsRef<str>) -> String {
    const LIMIT: usize = 120;
    let value = value.as_ref().replace('\n', " ");
    if value.chars().count() <= LIMIT {
        return value;
    }
    let shortened: String = value.chars().take(LIMIT).collect();
    format!("{shortened}...")
}

fn api_error_status(err: &rust_genai::Error) -> Option<u16> {
    match err {
        rust_genai::Error::ApiError { status, .. } => Some(*status),
        _ => None,
    }
}

fn print_results(results: &[StepResult]) {
    println!("== rust-genai live smoke ==");
    for result in results {
        let status = if result.ok { "PASS" } else { "FAIL" };
        println!("[{status}] {} :: {}", result.name, result.detail);
    }
    let passed = results.iter().filter(|item| item.ok).count();
    println!("== score {passed}/{} passed ==", results.len());
}

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let include_edge_probes = env_flag("GENAI_SMOKE_INCLUDE_EDGE_PROBES");
    let mut results = Vec::<StepResult>::new();

    let listed_models = client.models().all().await?;
    let generation_model = choose_generation_model(&listed_models).ok_or_else(|| {
        rust_genai::Error::InvalidConfig {
            message: "No low-cost text generation model found in discovered model set".into(),
        }
    })?;
    let embedding_model =
        choose_embedding_model(&listed_models).ok_or_else(|| rust_genai::Error::InvalidConfig {
            message: "No embedding model found in discovered model set".into(),
        })?;
    let preview_models: Vec<String> = listed_models
        .iter()
        .filter_map(|model| {
            model
                .name
                .as_deref()
                .map(bare_model_name)
                .map(str::to_string)
        })
        .take(8)
        .collect();
    results.push(StepResult::pass(
        "models.all",
        format!(
            "{} models, text={}, embed={}, sample={}",
            listed_models.len(),
            generation_model,
            embedding_model,
            preview_models.join(", ")
        ),
    ));

    let model = client.models().get(&generation_model).await?;
    let supported = model.supported_actions.unwrap_or_default().join(",");
    results.push(StepResult::pass(
        "models.get",
        format!("{generation_model} actions={supported}"),
    ));

    let response = client
        .models()
        .generate_content(
            &generation_model,
            vec![Content::text("Reply with exactly OK.")],
        )
        .await?;
    results.push(StepResult::pass(
        "models.generate_content",
        shorten(response.text().unwrap_or_else(|| "<empty>".to_string())),
    ));

    let mut stream = client
        .models()
        .generate_content_stream(
            &generation_model,
            vec![Content::text(
                "Reply with exactly three lowercase words about rust.",
            )],
            GenerateContentConfig::default(),
        )
        .await?;
    let mut joined = String::new();
    let mut chunks = 0usize;
    while let Some(item) = stream.next().await {
        let chunk = item?;
        chunks += 1;
        if let Some(text) = chunk.text() {
            joined.push_str(&text);
        }
    }
    results.push(StepResult::pass(
        "models.generate_content_stream",
        format!("chunks={chunks}, text={}", shorten(joined)),
    ));

    let token_response = client
        .models()
        .count_tokens(&generation_model, vec![Content::text("hello from rust")])
        .await?;
    results.push(StepResult::pass(
        "models.count_tokens",
        format!("total_tokens={:?}", token_response.total_tokens),
    ));

    let chat = client.chats().create(&generation_model);
    let first_turn = chat
        .send_message("Remember the word kiwi and reply with ACK.")
        .await?;
    results.push(StepResult::pass(
        "chats.send_message.first_turn",
        shorten(first_turn.text().unwrap_or_else(|| "<empty>".to_string())),
    ));
    let second_turn = chat.send_message("What word should you remember?").await?;
    results.push(StepResult::pass(
        "chats.send_message.second_turn",
        shorten(second_turn.text().unwrap_or_else(|| "<empty>".to_string())),
    ));

    let embedding_response = client
        .models()
        .embed_content(
            &embedding_model,
            vec![Content::text("hello from rust sdk smoke test")],
        )
        .await?;
    let dimensions = embedding_response
        .embeddings
        .as_ref()
        .and_then(|items| items.first())
        .and_then(|item| item.values.as_ref())
        .map(Vec::len);
    results.push(StepResult::pass(
        "models.embed_content",
        format!("model={embedding_model}, dims={dimensions:?}"),
    ));

    let upload_body = format!(
        "rust-genai smoke {}\nhello from e2e",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_secs()
    );
    let file = client
        .files()
        .upload(upload_body.as_bytes().to_vec(), "text/plain")
        .await?;
    let file_name = match file.name.clone() {
        Some(name) => name,
        None => {
            let detail = format!(
                "upload response missing file.name, mime={:?}, state={:?}",
                file.mime_type, file.state
            );
            results.push(StepResult::fail("files.upload", detail.clone()));
            print_results(&results);
            return Err(rust_genai::Error::Parse { message: detail });
        }
    };
    results.push(StepResult::pass(
        "files.upload",
        format!(
            "name={file_name}, mime={:?}, state={:?}",
            file.mime_type, file.state
        ),
    ));
    let mut file_probe_error: Option<rust_genai::Error> = None;

    match client
        .files()
        .wait_for_active(
            &file_name,
            WaitForFileConfig {
                poll_interval: Duration::from_secs(1),
                timeout: Some(Duration::from_secs(10)),
            },
        )
        .await
    {
        Ok(file) => results.push(StepResult::pass(
            "files.get",
            format!("state={:?}, size_bytes={:?}", file.state, file.size_bytes),
        )),
        Err(err) => {
            results.push(StepResult::fail("files.get", err.to_string()));
            file_probe_error = Some(err);
        }
    }

    if file_probe_error.is_none() {
        match client.files().list().await {
            Ok(list_response) => results.push(StepResult::pass(
                "files.list",
                format!(
                    "count_on_first_page={}",
                    list_response.files.unwrap_or_default().len()
                ),
            )),
            Err(err) => {
                results.push(StepResult::fail("files.list", err.to_string()));
                file_probe_error = Some(err);
            }
        }
    }

    if file_probe_error.is_none() && include_edge_probes {
        match client
            .models()
            .embed_content(
                "text-embedding-004",
                vec![Content::text("legacy example probe")],
            )
            .await
        {
            Ok(_) => results.push(StepResult::fail(
                "edge.legacy_text_embedding_004",
                "Legacy model succeeded; example assumptions changed",
            )),
            Err(err) => {
                let detail = err.to_string();
                if api_error_status(&err) == Some(404) {
                    results.push(StepResult::pass(
                        "edge.legacy_text_embedding_004",
                        format!("404 confirmed for text-embedding-004 on v1beta ({detail})"),
                    ));
                } else {
                    results.push(StepResult::fail("edge.legacy_text_embedding_004", detail));
                }
            }
        }

        match client.files().download(&file_name).await {
            Ok(bytes) => results.push(StepResult::fail(
                "edge.files.download_uploaded",
                format!("uploaded file became downloadable: {} bytes", bytes.len()),
            )),
            Err(err) => {
                let detail = err.to_string();
                if api_error_status(&err) == Some(400) {
                    results.push(StepResult::pass(
                        "edge.files.download_uploaded",
                        format!("uploaded files stay non-downloadable on Gemini API ({detail})"),
                    ));
                } else {
                    results.push(StepResult::fail("edge.files.download_uploaded", detail));
                }
            }
        }
    }

    match client.files().delete(&file_name).await {
        Ok(_) => results.push(StepResult::pass(
            "files.delete",
            format!("deleted {file_name}"),
        )),
        Err(err) => {
            results.push(StepResult::fail("files.delete", err.to_string()));
            if file_probe_error.is_none() {
                file_probe_error = Some(err);
            }
        }
    }

    print_results(&results);

    if let Some(err) = file_probe_error {
        return Err(err);
    }

    if let Some(failed) = results.iter().find(|item| !item.ok) {
        return Err(rust_genai::Error::InvalidConfig {
            message: format!("Live smoke failed at {}", failed.name),
        });
    }

    Ok(())
}
