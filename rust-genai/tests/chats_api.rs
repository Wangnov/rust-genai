use futures_util::StreamExt;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::afc::InlineCallableTool;
use rust_genai::types::models::{AutomaticFunctionCallingConfig, GenerateContentConfig};
use rust_genai::types::tool::FunctionDeclaration;

mod support;
use support::build_gemini_client;

#[tokio::test]
async fn chat_send_message_updates_history() {
    let server = MockServer::start().await;
    let response_body = json!({
        "candidates": [
            {"content": {"role": "model", "parts": [{"text": "Hi"}]}}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let chat = client.chats().create("gemini-2.0-flash");
    let response = chat.send_message("hello").await.unwrap();
    assert_eq!(response.text().as_deref(), Some("Hi"));

    let history = chat.history().await;
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn chat_send_alias_updates_history() {
    let server = MockServer::start().await;
    let response_body = json!({
        "candidates": [
            {"content": {"role": "model", "parts": [{"text": "Hi"}]}}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let chat = client.chats().create("gemini-2.0-flash");
    let response = chat.send("hello").await.unwrap();
    assert_eq!(response.text().as_deref(), Some("Hi"));

    let history = chat.history().await;
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn chat_send_message_without_content_does_not_append_history() {
    let server = MockServer::start().await;
    let response_body = json!({
        "candidates": [
            {}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let chat = client.chats().create("gemini-2.0-flash");
    let response = chat.send_message("hello").await.unwrap();
    assert!(response.text().is_none());

    let history = chat.history().await;
    assert_eq!(history.len(), 1);
}

#[tokio::test]
async fn chat_send_message_stream_updates_history() {
    let server = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]}}]}\n\n",
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"World\"}]}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.0-flash:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let chat = client.chats().create("gemini-2.0-flash");
    let stream = chat.send_message_stream("hi").await.unwrap();
    futures_util::pin_mut!(stream);
    let mut texts = Vec::new();
    while let Some(item) = stream.next().await {
        if let Some(text) = item.unwrap().text() {
            texts.push(text);
        }
    }
    assert_eq!(texts, vec!["Hello".to_string(), "World".to_string()]);

    let history = chat.history().await;
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].first_text(), Some("World"));
}

#[tokio::test]
async fn chat_send_stream_alias_without_content_keeps_history() {
    let server = MockServer::start().await;
    let sse_body = concat!("data: {\"candidates\":[{}]}\n\n", "data: [DONE]\n\n");

    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.0-flash:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let chat = client.chats().create("gemini-2.0-flash");
    let stream = chat.send_stream("hi").await.unwrap();
    futures_util::pin_mut!(stream);
    while let Some(item) = stream.next().await {
        item.unwrap();
    }

    let history = chat.history().await;
    assert_eq!(history.len(), 1);
}

#[tokio::test]
async fn chat_send_message_with_callable_tools() {
    let server = MockServer::start().await;
    let function_call_body = json!({
        "candidates": [
            {"content": {"role": "model", "parts": [{"functionCall": {"name": "echo", "args": {"msg": "hi"}}}]}}
        ]
    });
    let final_body = json!({
        "candidates": [
            {"content": {"role": "model", "parts": [{"text": "done"}]}}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .and(body_string_contains("functionResponse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(final_body))
        .with_priority(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(function_call_body))
        .with_priority(2)
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let chat = client.chats().create("gemini-2.0-flash");
    let mut tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "echo".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);
    tool.register_handler("echo", |args| async move { Ok(args) });

    let response = chat
        .send_message_with_callable_tools("hi", vec![Box::new(tool)])
        .await
        .unwrap();
    assert_eq!(response.text().as_deref(), Some("done"));

    let history = chat.history().await;
    assert!(history.len() >= 3);
}

#[tokio::test]
async fn chat_send_message_with_callable_tools_applies_afc_history() {
    let server = MockServer::start().await;
    let function_call_body = json!({
        "candidates": [
            {"content": {"role": "model", "parts": [{"functionCall": {"name": "echo", "args": {"msg": "hi"}}}]}}
        ]
    });
    let final_body = json!({
        "automaticFunctionCallingHistory": [
            {"role": "user", "parts": [{"text": "hi"}]}
        ],
        "candidates": [
            {"content": {"role": "model", "parts": [{"text": "done"}]}}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .and(body_string_contains("functionResponse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(final_body))
        .with_priority(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(function_call_body))
        .with_priority(2)
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let chat = client.chats().create("gemini-2.0-flash");
    let mut tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "echo".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);
    tool.register_handler("echo", |args| async move { Ok(args) });

    let response = chat
        .send_message_with_callable_tools("hi", vec![Box::new(tool)])
        .await
        .unwrap();
    assert_eq!(response.text().as_deref(), Some("done"));

    let history = chat.history().await;
    assert!(history.len() >= 3);
    assert_eq!(history[0].first_text(), Some("hi"));
    assert_eq!(history.last().unwrap().first_text(), Some("done"));
}

#[tokio::test]
async fn chat_send_message_stream_with_callable_tools_applies_afc_history() {
    let server = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"automaticFunctionCallingHistory\":[{\"role\":\"user\",\"parts\":[{\"text\":\"hi\"}]}],",
        "\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"done\"}]}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.0-flash:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let chat = client.chats().create("gemini-2.0-flash");
    let stream = chat
        .send_message_stream_with_callable_tools("hi", vec![])
        .await
        .unwrap();
    futures_util::pin_mut!(stream);
    while let Some(item) = stream.next().await {
        item.unwrap();
    }

    let history = chat.history().await;
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].first_text(), Some("hi"));
    assert_eq!(history[1].first_text(), Some("done"));
}

#[tokio::test]
async fn chat_send_message_stream_with_callable_tools() {
    let server = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"functionCall\":{\"name\":\"echo\",\"args\":{\"msg\":\"hi\"}}}]}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.0-flash:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&server)
        .await;

    let config = GenerateContentConfig {
        automatic_function_calling: Some(AutomaticFunctionCallingConfig {
            maximum_remote_calls: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    };
    let client = build_gemini_client(&server.uri());
    let chat = client
        .chats()
        .create_with_config("gemini-2.0-flash", config);

    let mut tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "echo".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);
    tool.register_handler("echo", |args| async move { Ok(args) });

    let stream = chat
        .send_message_stream_with_callable_tools("hi", vec![Box::new(tool)])
        .await
        .unwrap();
    futures_util::pin_mut!(stream);
    let mut seen = 0;
    while let Some(item) = stream.next().await {
        item.unwrap();
        seen += 1;
    }
    assert!(seen >= 1);

    let history = chat.history().await;
    assert!(!history.is_empty());
    assert_eq!(
        history.last().unwrap().role,
        Some(rust_genai::types::content::Role::Function)
    );
}

#[tokio::test]
async fn chat_clear_history_removes_entries() {
    let server = MockServer::start().await;
    let response_body = json!({
        "candidates": [
            {"content": {"role": "model", "parts": [{"text": "Hi"}]}}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&server)
        .await;

    let client = build_gemini_client(&server.uri());
    let chat = client.chats().create("gemini-2.0-flash");
    chat.send_message("hello").await.unwrap();
    assert_eq!(chat.history().await.len(), 2);

    chat.clear_history().await;
    assert!(chat.history().await.is_empty());
}
