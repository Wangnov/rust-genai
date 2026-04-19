use super::*;
use crate::client::{Backend, Client};
use crate::test_support::{
    test_client_inner_with_base as test_inner_with_base, test_vertex_inner_missing_config,
};
use futures_util::StreamExt;
use rust_genai_types::config::{GenerationConfig, ThinkingConfig};
use rust_genai_types::content::{
    Content, FunctionCall, FunctionResponse, FunctionResponseBlob, FunctionResponsePart, Part, Role,
};
use rust_genai_types::enums::{FunctionCallingMode, ThinkingLevel};
use rust_genai_types::http::HttpOptions as TypesHttpOptions;
use rust_genai_types::models::{
    AutomaticFunctionCallingConfig, ComputeTokensConfig, EditImageConfig, GenerateContentConfig,
    GenerateImagesConfig, GenerateVideosConfig, GenerateVideosSource, Image, RecontextImageConfig,
    RecontextImageSource, ReferenceImage, SegmentImageConfig, SegmentImageSource,
    UpscaleImageConfig,
};
use rust_genai_types::tool::{
    CodeExecution, FunctionCallingConfig, FunctionDeclaration, Tool, ToolConfig,
};
use serde::Deserialize;
use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_function_call_content() {
    let calls = vec![FunctionCall {
        id: Some("1".to_string()),
        name: Some("tool".to_string()),
        args: Some(json!({"x": 1})),
        partial_args: None,
        will_continue: None,
    }];
    let content = build_function_call_content(&calls);
    assert_eq!(content.role, Some(Role::Model));
    assert_eq!(content.parts.len(), 1);
}

#[tokio::test]
async fn test_compute_tokens_invalid_backend() {
    let client = Client::new("test-key").unwrap();
    let models = client.models();
    let err = models
        .compute_tokens("gemini-3-flash-preview", vec![Content::text("hi")])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidConfig { .. }));
}

async fn mount_vertex_model_mocks(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/textembedding-gecko:predict",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "predictions": [{"embeddings": {"values": [0.1, 0.2]}}],
            "metadata": {"billableCharacterCount": 12}
        })))
        .mount(server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/gemini-1.5-pro:countTokens",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "totalTokens": 2
        })))
        .mount(server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/gemini-1.5-pro:computeTokens",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tokensInfo": [{"role": "user", "tokenIds": [1, 2], "tokens": ["a", "b"]}]
        })))
        .mount(server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/imagen-generate:predict",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "predictions": [{"bytesBase64Encoded": "AQID", "mimeType": "image/png"}]
        })))
        .mount(server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/imagen-edit:predict",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "predictions": [{"bytesBase64Encoded": "AQID", "mimeType": "image/png"}]
        })))
        .mount(server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/imagen-upscale:predict",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "predictions": [{"bytesBase64Encoded": "AQID", "mimeType": "image/png"}]
        })))
        .mount(server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/imagen-recontext:predict",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "predictions": [{"bytesBase64Encoded": "AQID", "mimeType": "image/png"}]
        })))
        .mount(server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/imagen-segment:predict",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "predictions": [{"bytesBase64Encoded": "AQID", "mimeType": "image/png"}]
        })))
        .mount(server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/veo-vertex:predictLongRunning",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "operations/vertex-1",
            "response": {
                "videos": [
                    {
                        "gcsUri": "gs://example/video.mp4",
                        "mimeType": "video/mp4",
                        "bytesBase64Encoded": "AQID"
                    }
                ]
            }
        })))
        .mount(server)
        .await;
}

async fn assert_vertex_text_ops(models: &Models) {
    let embed = models
        .embed_content("textembedding-gecko", vec![Content::text("hi")])
        .await
        .unwrap();
    assert!(embed.embeddings.is_some());

    let count = models
        .count_tokens("gemini-1.5-pro", vec![Content::text("hi")])
        .await
        .unwrap();
    assert_eq!(count.total_tokens, Some(2));

    let compute = models
        .compute_tokens("gemini-1.5-pro", vec![Content::text("hi")])
        .await
        .unwrap();
    assert_eq!(compute.tokens_info.as_ref().unwrap().len(), 1);
}

async fn assert_vertex_image_ops(models: &Models) {
    let images = models
        .generate_images("imagen-generate", "prompt", GenerateImagesConfig::default())
        .await
        .unwrap();
    assert_eq!(images.generated_images.len(), 1);

    let reference = ReferenceImage {
        reference_image: Some(Image {
            image_bytes: Some(vec![1, 2, 3]),
            mime_type: Some("image/png".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let edit = models
        .edit_image(
            "imagen-edit",
            "prompt",
            vec![reference],
            EditImageConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(edit.generated_images.len(), 1);

    let image = Image {
        image_bytes: Some(vec![1, 2, 3]),
        mime_type: Some("image/png".to_string()),
        ..Default::default()
    };
    let upscale = models
        .upscale_image(
            "imagen-upscale",
            image.clone(),
            "x2",
            UpscaleImageConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(upscale.generated_images.len(), 1);

    let recontext = models
        .recontext_image(
            "imagen-recontext",
            RecontextImageSource {
                prompt: Some("hi".to_string()),
                ..Default::default()
            },
            RecontextImageConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(recontext.generated_images.len(), 1);

    let segment = models
        .segment_image(
            "imagen-segment",
            SegmentImageSource {
                image: Some(image),
                ..Default::default()
            },
            SegmentImageConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(segment.generated_masks.len(), 1);

    let op = models
        .generate_videos(
            "veo-vertex",
            GenerateVideosSource {
                prompt: Some("video".to_string()),
                ..Default::default()
            },
            GenerateVideosConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(op.name.as_deref(), Some("operations/vertex-1"));
    assert_eq!(op.response.as_ref().unwrap().generated_videos.len(), 1);
    let video = op.response.as_ref().unwrap().generated_videos[0]
        .video
        .as_ref()
        .unwrap();
    assert_eq!(video.uri.as_deref(), Some("gs://example/video.mp4"));
    assert_eq!(video.mime_type.as_deref(), Some("video/mp4"));
    assert_eq!(video.video_bytes.as_deref(), Some(&[1, 2, 3][..]));
}

#[tokio::test]
async fn test_models_vertex_api_methods() {
    let server = MockServer::start().await;
    mount_vertex_model_mocks(&server).await;

    let inner = test_inner_with_base(Backend::VertexAi, &server.uri(), "v1beta1");
    let models = Models::new(Arc::new(inner));
    assert_vertex_text_ops(&models).await;
    assert_vertex_image_ops(&models).await;

    let gemini = Client::new("test-key").unwrap();
    let err = gemini
        .models()
        .edit_image(
            "gemini-3-flash-preview",
            "prompt",
            vec![ReferenceImage::default()],
            EditImageConfig::default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidConfig { .. }));
}

#[tokio::test]
async fn test_models_validation_errors() {
    let client = Client::new("test-key").unwrap();
    let models = client.models();

    let response_part = FunctionResponsePart {
        inline_data: Some(FunctionResponseBlob {
            mime_type: "image/png".to_string(),
            data: vec![1, 2, 3],
            display_name: None,
        }),
        file_data: None,
    };
    let function_response = FunctionResponse {
        will_continue: None,
        scheduling: None,
        parts: Some(vec![response_part]),
        id: None,
        name: Some("tool".to_string()),
        response: None,
    };
    let content = Content::from_parts(
        vec![Part::function_response(function_response)],
        Role::Function,
    );
    let err = models
        .generate_content_with_config(
            "gemini-2.5-flash",
            vec![content],
            GenerateContentConfig::default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidConfig { .. }));

    let tool = Tool {
        code_execution: Some(CodeExecution::default()),
        ..Default::default()
    };
    let config = GenerateContentConfig {
        tools: Some(vec![tool]),
        ..Default::default()
    };
    let contents = vec![Content::from_parts(
        vec![Part::inline_data(vec![9], "image/png")],
        Role::User,
    )];
    let err = models
        .generate_content_with_config("gemini-2.5-flash", contents, config)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidConfig { .. }));
}

#[tokio::test]
async fn test_models_generate_content_vertex_and_errors() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/gemini-1.5-pro:generateContent",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "hello"}]}
            }]
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/gemini-error:generateContent",
        ))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let inner = test_inner_with_base(Backend::VertexAi, &server.uri(), "v1beta1");
    let models = Models::new(Arc::new(inner));

    let ok = models
        .generate_content_with_config(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            GenerateContentConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(ok.text(), Some("hello".to_string()));

    let err = models
        .generate_content_with_config(
            "gemini-error",
            vec![Content::text("hi")],
            GenerateContentConfig::default(),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));
}

#[tokio::test]
async fn test_generate_content_stream_uses_gemini_request_converter() {
    let server = MockServer::start().await;
    let expected_body = json!({
        "contents": [{
            "role": "user",
            "parts": [{"text": "hi"}]
        }],
        "systemInstruction": {
            "role": "user",
            "parts": [{"text": "system"}]
        },
        "generationConfig": {
            "temperature": 0.5,
            "responseMimeType": "application/json",
            "thinkingConfig": {
                "thinkingLevel": "HIGH",
                "includeThoughts": true
            }
        },
        "tools": [{
            "functionDeclarations": [{
                "name": "lookup_weather",
                "description": "Look up weather"
            }]
        }],
        "toolConfig": {
            "functionCallingConfig": {
                "allowedFunctionNames": ["lookup_weather"],
                "mode": "AUTO",
                "streamFunctionCallArguments": true
            }
        },
        "labels": {
            "suite": "stream"
        }
    });

    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-1.5-pro:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .and(body_json(expected_body))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]}}]}\n\n\
                     data: [DONE]\n\n",
                ),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();
    let config = stream_request_config();

    let mut stream = client
        .models()
        .generate_content_stream("gemini-1.5-pro", vec![Content::text("hi")], config)
        .await
        .unwrap();
    let first = stream.next().await.unwrap().unwrap();
    assert_eq!(first.text(), Some("ok".to_string()));
}

#[tokio::test]
async fn test_generate_content_stream_uses_vertex_request_converter() {
    let server = MockServer::start().await;
    let expected_body = json!({
        "contents": [{
            "role": "user",
            "parts": [{"text": "hi"}]
        }],
        "systemInstruction": {
            "role": "user",
            "parts": [{"text": "system"}]
        },
        "generationConfig": {
            "temperature": 0.5,
            "responseMimeType": "application/json",
            "thinkingConfig": {
                "thinkingLevel": "HIGH",
                "includeThoughts": true
            }
        },
        "tools": [{
            "functionDeclarations": [{
                "name": "lookup_weather",
                "description": "Look up weather"
            }]
        }],
        "toolConfig": {
            "functionCallingConfig": {
                "allowedFunctionNames": ["lookup_weather"],
                "mode": "AUTO",
                "streamFunctionCallArguments": true
            }
        },
        "labels": {
            "suite": "stream"
        }
    });

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/gemini-1.5-pro:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .and(body_json(expected_body))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"ok\"}]}}]}\n\n\
                     data: [DONE]\n\n",
                ),
        )
        .mount(&server)
        .await;

    let inner = test_inner_with_base(Backend::VertexAi, &server.uri(), "v1beta1");
    let models = Models::new(Arc::new(inner));
    let config = stream_request_config();

    let mut stream = models
        .generate_content_stream("gemini-1.5-pro", vec![Content::text("hi")], config)
        .await
        .unwrap();
    let first = stream.next().await.unwrap().unwrap();
    assert_eq!(first.text(), Some("ok".to_string()));
}

fn stream_request_config() -> GenerateContentConfig {
    GenerateContentConfig {
        system_instruction: Some(Content::text("system")),
        generation_config: Some(GenerationConfig {
            temperature: Some(0.5),
            response_mime_type: Some("application/json".into()),
            thinking_config: Some(ThinkingConfig {
                thinking_level: Some(ThinkingLevel::High),
                include_thoughts: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }),
        tools: Some(vec![Tool {
            function_declarations: Some(vec![FunctionDeclaration {
                name: "lookup_weather".into(),
                description: Some("Look up weather".into()),
                parameters: None,
                parameters_json_schema: None,
                response: None,
                response_json_schema: None,
                behavior: None,
            }]),
            ..Default::default()
        }]),
        tool_config: Some(ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                allowed_function_names: Some(vec!["lookup_weather".into()]),
                mode: Some(FunctionCallingMode::Auto),
                stream_function_call_arguments: Some(true),
            }),
            ..Default::default()
        }),
        labels: Some(std::collections::HashMap::from([(
            "suite".to_string(),
            "stream".to_string(),
        )])),
        ..Default::default()
    }
}

#[derive(Debug, Deserialize, PartialEq)]
struct JsonSmokeResponse {
    ok: bool,
}

#[tokio::test]
async fn test_generate_json_parses_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "{\"ok\":true}"}]
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();

    let parsed = client
        .models()
        .generate_json::<JsonSmokeResponse>("gemini-1.5-pro", vec![Content::text("return json")])
        .await
        .unwrap();

    assert_eq!(parsed, JsonSmokeResponse { ok: true });
}

#[tokio::test]
async fn test_generate_json_parses_concatenated_text_parts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "{\"ok\":"},
                        {"text": "true}"},
                        {"functionCall": {"name": "ignored_helper", "args": {}}}
                    ]
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();

    let parsed = client
        .models()
        .generate_json::<JsonSmokeResponse>("gemini-1.5-pro", vec![Content::text("return json")])
        .await
        .unwrap();

    assert_eq!(parsed, JsonSmokeResponse { ok: true });
}

#[tokio::test]
async fn test_generate_json_requires_text_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"inlineData": {"mimeType": "image/png", "data": "Zm9v"}}]
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();

    let err = client
        .models()
        .generate_json::<JsonSmokeResponse>("gemini-1.5-pro", vec![Content::text("return json")])
        .await
        .unwrap_err();

    assert!(matches!(err, Error::Parse { .. }));
}

#[tokio::test]
async fn test_generate_json_rejects_invalid_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "{invalid json"}]
                }
            }]
        })))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();

    let err = client
        .models()
        .generate_json::<JsonSmokeResponse>("gemini-1.5-pro", vec![Content::text("return json")])
        .await
        .unwrap_err();

    assert!(matches!(err, Error::Serialization { .. }));
}

#[tokio::test]
async fn test_generate_content_event_stream_emits_text_response_and_done() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-1.5-pro:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-test", "1")
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]}}]}\n\n\
                     data: [DONE]\n\n",
                ),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();
    let mut stream = client
        .models()
        .generate_content_event_stream(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            GenerateContentConfig::default(),
        )
        .await
        .unwrap();

    let first = stream.next_event().await.unwrap().unwrap();
    let second = stream.next_event().await.unwrap().unwrap();
    let third = stream.next_event().await.unwrap().unwrap();
    let fourth = stream.next_event().await.unwrap();

    assert!(matches!(first, GenerateContentStreamEvent::Text(ref text) if text == "Hi"));
    assert!(matches!(
        second,
        GenerateContentStreamEvent::Response(ref response)
            if response.text() == Some("Hi".to_string())
                && response
                    .sdk_http_response
                    .as_ref()
                    .and_then(|http| http.headers.as_ref())
                    .and_then(|headers| headers.get("x-test"))
                    .map(String::as_str)
                    == Some("1")
    ));
    assert!(matches!(
        third,
        GenerateContentStreamEvent::Done(ref response)
            if response.text() == Some("Hi".to_string())
                && response
                    .sdk_http_response
                    .as_ref()
                    .and_then(|http| http.headers.as_ref())
                    .and_then(|headers| headers.get("x-test"))
                    .map(String::as_str)
                    == Some("1")
    ));
    assert!(fourth.is_none());
}

#[tokio::test]
async fn test_generate_content_event_stream_skips_done_on_plain_eof() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-1.5-pro:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi\"}]}}]}\n\n",
                ),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();
    let mut stream = client
        .models()
        .generate_content_event_stream(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            GenerateContentConfig::default(),
        )
        .await
        .unwrap();

    let first = stream.next_event().await.unwrap().unwrap();
    let second = stream.next_event().await.unwrap().unwrap();
    let third = stream.next_event().await.unwrap();

    assert!(matches!(first, GenerateContentStreamEvent::Text(ref text) if text == "Hi"));
    assert!(matches!(
        second,
        GenerateContentStreamEvent::Response(ref response) if response.text() == Some("Hi".to_string())
    ));
    assert!(third.is_none());
}

#[tokio::test]
async fn test_generate_content_event_stream_emits_function_call_and_usage() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta/models/gemini-1.5-pro:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"functionCall\":{\"name\":\"lookup\",\"args\":{\"q\":\"rust\"}}}]}}],\"usageMetadata\":{\"promptTokenCount\":3,\"totalTokenCount\":5}}\n\n\
                     data: [DONE]\n\n",
                ),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-key")
        .base_url(server.uri())
        .build()
        .unwrap();
    let mut stream = client
        .models()
        .generate_content_event_stream(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            GenerateContentConfig::default(),
        )
        .await
        .unwrap();

    let first = stream.next_event().await.unwrap().unwrap();
    let second = stream.next_event().await.unwrap().unwrap();
    let third = stream.next_event().await.unwrap().unwrap();
    let fourth = stream.next_event().await.unwrap().unwrap();

    assert!(matches!(
        first,
        GenerateContentStreamEvent::FunctionCall(ref call)
            if call.name.as_deref() == Some("lookup")
    ));
    assert!(matches!(
        second,
        GenerateContentStreamEvent::Usage(ref usage)
            if usage.prompt_token_count == Some(3) && usage.total_token_count == Some(5)
    ));
    assert!(matches!(
        third,
        GenerateContentStreamEvent::Response(ref response) if response.function_calls().len() == 1
    ));
    assert!(matches!(
        fourth,
        GenerateContentStreamEvent::Done(ref response) if response.function_calls().len() == 1
    ));
}

#[tokio::test]
async fn test_generate_content_stream_thought_signature_error() {
    let client = Client::new("test-key").unwrap();
    let models = client.models();
    let contents = vec![
        Content::user("hi"),
        Content::from_parts(
            vec![Part::function_call(FunctionCall {
                id: None,
                name: Some("tool".to_string()),
                args: None,
                partial_args: None,
                will_continue: None,
            })],
            Role::Model,
        ),
    ];
    let err = models
        .generate_content_stream(
            "gemini-3-pro-preview",
            contents,
            GenerateContentConfig::default(),
        )
        .await
        .err()
        .unwrap();
    assert!(matches!(err, Error::MissingThoughtSignature { .. }));
}

#[tokio::test]
async fn test_compute_tokens_error_response_and_extra_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/gemini-1.5-pro:computeTokens",
        ))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let inner = test_inner_with_base(Backend::VertexAi, &server.uri(), "v1beta1");
    let models = Models::new(Arc::new(inner));
    let err = models
        .compute_tokens_with_config(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            ComputeTokensConfig {
                http_options: Some(rust_genai_types::http::HttpOptions {
                    extra_body: Some(json!("bad")),
                    ..Default::default()
                }),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::InvalidConfig { .. }));

    let err = models
        .compute_tokens("gemini-1.5-pro", vec![Content::text("hi")])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));
}

#[tokio::test]
async fn test_generate_content_callable_tools_invalid_afc_config() {
    let client = Client::new("test-key").unwrap();
    let models = client.models();
    let mut tool = crate::afc::InlineCallableTool::from_declarations(vec![FunctionDeclaration {
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
        tool_config: Some(rust_genai_types::tool::ToolConfig {
            function_calling_config: Some(rust_genai_types::tool::FunctionCallingConfig {
                stream_function_call_arguments: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }),
        automatic_function_calling: Some(AutomaticFunctionCallingConfig {
            disable: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };

    let err = models
        .generate_content_with_callable_tools(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            config,
            vec![Box::new(tool)],
        )
        .await
        .err()
        .unwrap();
    assert!(matches!(err, Error::InvalidConfig { .. }));
}

#[tokio::test]
async fn test_models_generate_content_stream_callable_tools_error() {
    let inner = test_vertex_inner_missing_config();
    let models = Models::new(Arc::new(inner));

    let mut tool = crate::afc::InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "test_tool".to_string(),
        description: None,
        parameters: None,
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);
    tool.register_handler("test_tool", |_value| async move { Ok(json!({"ok": true})) });

    let mut stream = models
        .generate_content_stream_with_callable_tools(
            "gemini-1.5-pro",
            vec![Content::text("hi")],
            GenerateContentConfig::default(),
            vec![Box::new(tool)],
        )
        .await
        .unwrap();

    let err = stream.next().await.unwrap().unwrap_err();
    assert!(matches!(err, Error::InvalidConfig { .. }));
}

#[tokio::test]
async fn test_models_token_estimator_local() {
    struct DummyEstimator;
    impl TokenEstimator for DummyEstimator {
        fn estimate_tokens(&self, _contents: &[Content]) -> usize {
            7
        }
    }

    let client = Client::new("test-key").unwrap();
    let models = client.models();
    let contents = vec![Content::text("hi")];
    let estimator = DummyEstimator;

    let local = models.estimate_tokens_local(&contents, &estimator);
    assert_eq!(local.total_tokens, Some(7));

    let local_with_config = models.estimate_tokens_local_with_config(
        &contents,
        &CountTokensConfig::default(),
        &estimator,
    );
    assert_eq!(local_with_config.total_tokens, Some(7));

    let estimated = models
        .count_tokens_or_estimate(
            "gemini-1.5-pro",
            contents,
            CountTokensConfig::default(),
            Some(&estimator),
        )
        .await
        .unwrap();
    assert_eq!(estimated.total_tokens, Some(7));
}

#[tokio::test]
async fn test_models_vertex_image_methods_error_with_extra_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(
            "/v1beta1/projects/proj/locations/loc/publishers/google/models/imagen-3.0:predict",
        ))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let inner = test_inner_with_base(Backend::VertexAi, &server.uri(), "v1beta1");
    let models = Models::new(Arc::new(inner));
    let image = Image {
        image_bytes: Some(vec![1, 2, 3]),
        mime_type: Some("image/png".to_string()),
        ..Default::default()
    };

    let edit_config = EditImageConfig {
        http_options: Some(TypesHttpOptions {
            extra_body: Some(json!({"extra": true})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = models
        .edit_image(
            "imagen-3.0",
            "prompt",
            vec![ReferenceImage {
                reference_image: Some(image.clone()),
                ..Default::default()
            }],
            edit_config,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));

    let upscale_config = UpscaleImageConfig {
        http_options: Some(TypesHttpOptions {
            extra_body: Some(json!({"extra": true})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = models
        .upscale_image("imagen-3.0", image.clone(), "x2", upscale_config)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));

    let recontext_config = RecontextImageConfig {
        http_options: Some(TypesHttpOptions {
            extra_body: Some(json!({"extra": true})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = models
        .recontext_image(
            "imagen-3.0",
            RecontextImageSource {
                prompt: Some("scene".to_string()),
                person_image: Some(image.clone()),
                ..Default::default()
            },
            recontext_config,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));

    let segment_config = SegmentImageConfig {
        http_options: Some(TypesHttpOptions {
            extra_body: Some(json!({"extra": true})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = models
        .segment_image(
            "imagen-3.0",
            SegmentImageSource {
                prompt: Some("segment".to_string()),
                image: Some(image),
                ..Default::default()
            },
            segment_config,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ApiError { .. }));
}
