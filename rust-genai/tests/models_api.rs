mod support;

use futures_util::StreamExt;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::afc::InlineCallableTool;
use rust_genai::types::content::{Content, Role};
use rust_genai::types::http::HttpOptions as TypesHttpOptions;
use rust_genai::types::models::{
    AutomaticFunctionCallingConfig, CountTokensConfig, DeleteModelConfig, GenerateContentConfig,
    GenerateImagesConfig, GenerateVideosConfig, GenerateVideosSource, ListModelsConfig,
    UpdateModelConfig,
};
use rust_genai::types::tool::FunctionDeclaration;
use rust_genai::Error;

use support::build_gemini_client;

#[tokio::test]
async fn test_generate_content_gemini_api() {
    let mock_server = MockServer::start().await;
    let response_body = json!({
        "candidates": [
            {
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "Hello"}
                    ]
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());

    let response = client
        .models()
        .generate_content("gemini-2.0-flash", vec![Content::text("Test")])
        .await
        .unwrap();
    assert_eq!(response.text(), Some("Hello".to_string()));
}

#[tokio::test]
async fn test_generate_content_should_return_http_response() {
    let mock_server = MockServer::start().await;
    let response_body = json!({
        "candidates": [
            {
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "Hello"}
                    ]
                }
            }
        ]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-2.0-flash:generateContent"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-test", "1")
                .set_body_json(response_body.clone()),
        )
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let config = GenerateContentConfig {
        should_return_http_response: Some(true),
        ..Default::default()
    };
    let response = client
        .models()
        .generate_content_with_config("gemini-2.0-flash", vec![Content::text("Test")], config)
        .await
        .unwrap();

    assert!(response.candidates.is_empty());

    let sdk_http_response = response.sdk_http_response.unwrap();
    assert_eq!(
        sdk_http_response
            .headers
            .as_ref()
            .and_then(|headers| headers.get("x-test"))
            .map(String::as_str),
        Some("1")
    );
    let body = sdk_http_response.body.unwrap();
    let body_json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body_json, response_body);
}

#[tokio::test]
async fn test_generate_content_stream_should_return_http_response_rejected() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.0-flash:streamGenerateContent",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string("data: [DONE]\n\n"))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let config = GenerateContentConfig {
        should_return_http_response: Some(true),
        ..Default::default()
    };

    let result = client
        .models()
        .generate_content_stream("gemini-2.0-flash", vec![Content::text("Test")], config)
        .await;

    assert!(matches!(result, Err(Error::InvalidConfig { .. })));
}

#[tokio::test]
async fn test_sse_streaming() {
    let mock_server = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-2.0-flash:streamGenerateContent",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());

    let mut stream = client
        .models()
        .generate_content_stream(
            "gemini-2.0-flash",
            vec![Content::text("Test")],
            GenerateContentConfig::default(),
        )
        .await
        .unwrap();

    let mut texts = Vec::new();
    while let Some(item) = stream.next().await {
        let response = item.unwrap();
        if let Some(text) = response.text() {
            texts.push(text);
        }
    }

    assert_eq!(texts, vec!["Hi".to_string()]);
}

#[tokio::test]
async fn test_list_models_with_query_params() {
    let mock_server = MockServer::start().await;
    let response_body = json!({
        "models": []
    });

    Mock::given(method("GET"))
        .and(path("/v1beta/models"))
        .and(query_param("pageSize", "5"))
        .and(query_param("pageToken", "token-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());

    let _ = client
        .models()
        .list_with_config(ListModelsConfig {
            page_size: Some(5),
            page_token: Some("token-1".to_string()),
            filter: None,
            query_base: None,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn test_models_gemini_media_and_tokens() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:batchEmbedContents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "embeddings": [{"values": [0.1, 0.2]}]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:countTokens"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "totalTokens": 4
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/imagen-3.0:predict"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "predictions": [{"bytesBase64Encoded": "AQID", "mimeType": "image/png"}]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/veo-1.0:predictLongRunning"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "operations/1",
            "response": {"generateVideoResponse": {"ok": true}}
        })))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());

    let embed = client
        .models()
        .embed_content("gemini-1.5-pro", vec![Content::text("hi")])
        .await
        .unwrap();
    assert!(embed.embeddings.is_some());

    let count = client
        .models()
        .count_tokens("gemini-1.5-pro", vec![Content::text("hi")])
        .await
        .unwrap();
    assert_eq!(count.total_tokens, Some(4));

    let images = client
        .models()
        .generate_images("imagen-3.0", "prompt", GenerateImagesConfig::default())
        .await
        .unwrap();
    assert_eq!(images.generated_images.len(), 1);

    let op = client
        .models()
        .generate_videos(
            "veo-1.0",
            GenerateVideosSource {
                prompt: Some("video".to_string()),
                ..Default::default()
            },
            GenerateVideosConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(op.name.as_deref(), Some("operations/1"));
}

#[tokio::test]
async fn test_models_gemini_crud() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1beta/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "models": [{"name": "models/gemini-1.5-pro"}]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/models/gemini-1.5-pro"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "models/gemini-1.5-pro"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("PATCH"))
        .and(path("/v1beta/models/gemini-1.5-pro"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "models/gemini-1.5-pro",
            "displayName": "updated"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1beta/models/gemini-1.5-pro"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());

    let list = client.models().list().await.unwrap();
    assert_eq!(list.models.unwrap().len(), 1);

    let model = client.models().get("gemini-1.5-pro").await.unwrap();
    assert_eq!(model.name.as_deref(), Some("models/gemini-1.5-pro"));

    let updated = client
        .models()
        .update("gemini-1.5-pro", UpdateModelConfig::default())
        .await
        .unwrap();
    assert_eq!(updated.display_name.as_deref(), Some("updated"));

    client
        .models()
        .delete("gemini-1.5-pro", DeleteModelConfig::default())
        .await
        .unwrap();
}

#[tokio::test]
async fn test_models_generate_content_callable_tools() {
    let mock_server = MockServer::start().await;
    let function_call_body = json!({
        "candidates": [{
            "content": {"role": "model", "parts": [{
                "functionCall": {"name": "test_tool", "args": {"x": 1}}
            }]}
        }]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .and(body_string_contains("functionResponse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "done"}]}
            }]
        })))
        .with_priority(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(function_call_body))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());

    let mut tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "test_tool".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);
    tool.register_handler("test_tool", |value| async move {
        let x = value
            .get("x")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        Ok(json!({ "result": x + 1 }))
    });

    let config = GenerateContentConfig {
        automatic_function_calling: Some(AutomaticFunctionCallingConfig {
            maximum_remote_calls: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    };

    let response = client
        .models()
        .generate_content_with_callable_tools(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            config,
            vec![Box::new(tool)],
        )
        .await
        .unwrap();
    assert_eq!(response.text(), Some("done".to_string()));
    assert!(response.automatic_function_calling_history.is_some());
}

#[tokio::test]
async fn test_models_generate_content_stream_callable_tools() {
    let mock_server = MockServer::start().await;
    let payload = json!({
        "candidates": [{
            "content": {"role": "model", "parts": [{
                "functionCall": {"name": "test_tool", "args": {"x": 1}}
            }]}
        }]
    });
    let sse_body = format!("data: {payload}\n\ndata: [DONE]\n\n");

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:streamGenerateContent"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());

    let mut tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "test_tool".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);
    tool.register_handler("test_tool", |_value| async move { Ok(json!({"ok": true})) });

    let config = GenerateContentConfig {
        automatic_function_calling: Some(AutomaticFunctionCallingConfig {
            maximum_remote_calls: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut stream = client
        .models()
        .generate_content_stream_with_callable_tools(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            config,
            vec![Box::new(tool)],
        )
        .await
        .unwrap();

    let mut saw_function_role = false;
    while let Some(item) = stream.next().await {
        let response = item.unwrap();
        if response
            .candidates
            .first()
            .and_then(|c| c.content.as_ref())
            .and_then(|c| c.role)
            == Some(Role::Function)
        {
            saw_function_role = true;
        }
    }
    assert!(saw_function_role);
}

#[tokio::test]
async fn test_models_generate_content_callable_tools_disabled() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "disabled"}]}
            }]
        })))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let mut tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "test_tool".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);
    tool.register_handler("test_tool", |_value| async move { Ok(json!({"ok": true})) });

    let config = GenerateContentConfig {
        automatic_function_calling: Some(AutomaticFunctionCallingConfig {
            maximum_remote_calls: Some(0),
            ..Default::default()
        }),
        ..Default::default()
    };

    let response = client
        .models()
        .generate_content_with_callable_tools(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            config,
            vec![Box::new(tool)],
        )
        .await
        .unwrap();
    assert_eq!(response.text(), Some("disabled".to_string()));
}

#[tokio::test]
async fn test_models_generate_content_stream_callable_tools_disabled() {
    let mock_server = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"hi\"}]}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:streamGenerateContent"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let mut tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "test_tool".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);
    tool.register_handler("test_tool", |_value| async move { Ok(json!({"ok": true})) });

    let config = GenerateContentConfig {
        automatic_function_calling: Some(AutomaticFunctionCallingConfig {
            maximum_remote_calls: Some(0),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut stream = client
        .models()
        .generate_content_stream_with_callable_tools(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            config,
            vec![Box::new(tool)],
        )
        .await
        .unwrap();

    let mut texts = Vec::new();
    while let Some(item) = stream.next().await {
        let response = item.unwrap();
        if let Some(text) = response.text() {
            texts.push(text);
        }
    }
    assert_eq!(texts, vec!["hi".to_string()]);
}

#[tokio::test]
async fn test_generate_content_stream_error_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:streamGenerateContent"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let err = client
        .models()
        .generate_content_stream(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            GenerateContentConfig::default(),
        )
        .await
        .err()
        .unwrap();
    assert!(matches!(err, Error::ApiError { .. }));
}

#[tokio::test]
async fn test_models_embed_and_count_error_responses() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:batchEmbedContents"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:countTokens"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let err = client
        .models()
        .embed_content("gemini-1.5-pro", vec![Content::text("hi")])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));

    let err = client
        .models()
        .count_tokens("gemini-1.5-pro", vec![Content::text("hi")])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));
}

#[tokio::test]
async fn test_models_generate_content_callable_tools_empty() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "ok"}]}
            }]
        })))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let response = client
        .models()
        .generate_content_with_callable_tools(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            GenerateContentConfig::default(),
            vec![],
        )
        .await
        .unwrap();
    assert_eq!(response.text(), Some("ok".to_string()));
}

#[tokio::test]
async fn test_models_generate_content_callable_tools_initial_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "test_tool".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);

    let err = client
        .models()
        .generate_content_with_callable_tools(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            GenerateContentConfig::default(),
            vec![Box::new(tool)],
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));
}

#[tokio::test]
async fn test_models_generate_content_callable_tools_max_calls_break() {
    let mock_server = MockServer::start().await;
    let function_call_body = json!({
        "candidates": [{
            "content": {"role": "model", "parts": [{
                "functionCall": {"name": "test_tool", "args": {"x": 1}}
            }]}
        }]
    });

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .and(body_string_contains("functionResponse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(function_call_body.clone()))
        .with_priority(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(function_call_body))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let mut tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "test_tool".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);
    tool.register_handler("test_tool", |_value| async move { Ok(json!({"ok": true})) });

    let config = GenerateContentConfig {
        automatic_function_calling: Some(AutomaticFunctionCallingConfig {
            maximum_remote_calls: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    };

    let response = client
        .models()
        .generate_content_with_callable_tools(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            config,
            vec![Box::new(tool)],
        )
        .await
        .unwrap();
    assert!(response.automatic_function_calling_history.is_some());
    assert!(!response.function_calls().is_empty());
}

#[tokio::test]
async fn test_models_list_all_and_crud_errors() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1beta/models"))
        .and(query_param("pageSize", "0"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/models"))
        .and(query_param("pageSize", "2"))
        .and(query_param_is_missing("pageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "models": [{"name": "models/a"}],
            "nextPageToken": "next"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/models"))
        .and(query_param("pageSize", "2"))
        .and(query_param("pageToken", "next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "models": [{"name": "models/b"}]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/models/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;

    Mock::given(method("PATCH"))
        .and(path("/v1beta/models/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1beta/models/bad"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());

    let err = client
        .models()
        .list_with_config(ListModelsConfig {
            page_size: Some(0),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));

    let all = client
        .models()
        .all_with_config(ListModelsConfig {
            page_size: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(all.len(), 2);

    let err = client.models().get("bad").await.unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));

    let update_config = UpdateModelConfig {
        http_options: Some(TypesHttpOptions {
            extra_body: Some(json!({"extra": true})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = client
        .models()
        .update("bad", update_config)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));

    let err = client
        .models()
        .delete("bad", DeleteModelConfig::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));
}

#[tokio::test]
async fn test_models_generate_media_errors_with_extra_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/imagen-3.0:predict"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/models/veo-1.0:predictLongRunning"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());

    let img_config = GenerateImagesConfig {
        http_options: Some(TypesHttpOptions {
            extra_body: Some(json!({"extra": 1})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = client
        .models()
        .generate_images("imagen-3.0", "prompt", img_config)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));

    let vid_config = GenerateVideosConfig {
        http_options: Some(TypesHttpOptions {
            extra_body: Some(json!({"extra": 2})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = client
        .models()
        .generate_videos(
            "veo-1.0",
            GenerateVideosSource {
                prompt: Some("video".to_string()),
                ..Default::default()
            },
            vid_config,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));
}

#[tokio::test]
async fn test_models_count_tokens_or_estimate_remote() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:countTokens"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "totalTokens": 3
        })))
        .mount(&mock_server)
        .await;

    let client = build_gemini_client(&mock_server.uri());
    let counted = client
        .models()
        .count_tokens_or_estimate(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            CountTokensConfig::default(),
            None,
        )
        .await
        .unwrap();
    assert_eq!(counted.total_tokens, Some(3));
}
