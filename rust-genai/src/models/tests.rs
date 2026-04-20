use super::*;
use crate::client::{Backend, Client};
use crate::test_support::{
    test_client_inner_with_base as test_inner_with_base, test_vertex_inner_missing_config,
};
use futures_util::StreamExt;
use rust_genai_types::config::{GenerationConfig, ThinkingConfig};
use rust_genai_types::content::{
    Content, FunctionCall, FunctionResponse, FunctionResponseBlob, FunctionResponsePart, Part,
    PartMediaResolution, PartialArg, Role, VideoMetadata,
};
use rust_genai_types::enums::{FunctionCallingMode, ThinkingLevel};
use rust_genai_types::http::HttpOptions as TypesHttpOptions;
use rust_genai_types::http::HttpResponse;
use rust_genai_types::models::{
    AutomaticFunctionCallingConfig, ComputeTokensConfig, EditImageConfig, GenerateContentConfig,
    GenerateImagesConfig, GenerateVideosConfig, GenerateVideosSource, Image, RecontextImageConfig,
    RecontextImageSource, ReferenceImage, SegmentImageConfig, SegmentImageSource,
    UpscaleImageConfig,
};
use rust_genai_types::response::{
    Candidate, GenerateContentResponse, GenerateContentResponseUsageMetadata, PromptFeedback,
    SafetyRating, UrlContextMetadata, UrlMetadata,
};
use rust_genai_types::tool::{
    CodeExecution, FunctionCallingConfig, FunctionDeclaration, Tool, ToolConfig,
};
use rust_genai_types::{
    enums::{
        BlockedReason, FinishReason, HarmCategory, HarmProbability, PartMediaResolutionLevel,
        UrlRetrievalStatus,
    },
    grounding::{Citation, CitationMetadata, GroundingMetadata},
    logprobs::{LogprobCandidate, LogprobsResult, TopCandidates},
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
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
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
async fn test_generate_json_rejects_non_json_mime_type() {
    let client = Client::builder()
        .api_key("test-key")
        .base_url("http://localhost.invalid")
        .build()
        .unwrap();

    let err = client
        .models()
        .generate_json_with_config::<JsonSmokeResponse>(
            "gemini-1.5-pro",
            vec![Content::text("return json")],
            GenerateContentConfig {
                generation_config: Some(GenerationConfig {
                    response_mime_type: Some("text/plain".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        Error::InvalidConfig { ref message }
            if message.contains("response_mime_type = application/json")
    ));
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

#[cfg(feature = "schemars")]
#[tokio::test]
async fn test_generate_json_with_schema_sets_response_json_schema() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-1.5-pro:generateContent"))
        .and(wiremock::matchers::body_string_contains(
            "\"responseMimeType\":\"application/json\"",
        ))
        .and(wiremock::matchers::body_string_contains(
            "\"responseJsonSchema\"",
        ))
        .and(wiremock::matchers::body_string_contains(
            "\"required\":[\"ok\"]",
        ))
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
        .generate_json_with_schema::<JsonSmokeResponse>(
            "gemini-1.5-pro",
            vec![Content::text("return json")],
        )
        .await
        .unwrap();

    assert_eq!(parsed, JsonSmokeResponse { ok: true });
}

#[cfg(feature = "schemars")]
#[tokio::test]
async fn test_generate_json_with_schema_rejects_existing_schema_config() {
    let client = Client::builder()
        .api_key("test-key")
        .base_url("http://localhost.invalid")
        .build()
        .unwrap();

    let err = client
        .models()
        .generate_json_with_schema_with_config::<JsonSmokeResponse>(
            "gemini-1.5-pro",
            vec![Content::text("return json")],
            GenerateContentConfig {
                generation_config: Some(GenerationConfig {
                    response_schema: Some(rust_genai_types::tool::Schema::string()),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        Error::InvalidConfig { ref message }
            if message.contains("empty response schema configuration")
    ));
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
async fn test_generate_content_event_stream_done_is_aggregated() {
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
                    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hi \"}]}}]}\n\n\
                     data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"there\"}]}}]}\n\n\
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
    let fifth = stream.next_event().await.unwrap().unwrap();
    let sixth = stream.next_event().await.unwrap();

    assert!(matches!(first, GenerateContentStreamEvent::Text(ref text) if text == "Hi "));
    assert!(matches!(
        second,
        GenerateContentStreamEvent::Response(ref response)
            if response.text() == Some("Hi ".to_string())
    ));
    assert!(matches!(third, GenerateContentStreamEvent::Text(ref text) if text == "there"));
    assert!(matches!(
        fourth,
        GenerateContentStreamEvent::Response(ref response)
            if response.text() == Some("there".to_string())
    ));
    assert!(matches!(
        fifth,
        GenerateContentStreamEvent::Done(ref response)
            if response.text() == Some("Hi there".to_string())
    ));
    assert!(sixth.is_none());
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
async fn test_generate_content_event_stream_done_merges_function_call_fragments() {
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
                    "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"functionCall\":{\"id\":\"call-1\",\"name\":\"lookup\",\"partialArgs\":[{\"jsonPath\":\"$.city\",\"stringValue\":\"Bei\",\"willContinue\":true}],\"willContinue\":true}}]}}]}\n\n\
                     data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"functionCall\":{\"id\":\"call-1\",\"name\":\"lookup\",\"partialArgs\":[{\"jsonPath\":\"$.city\",\"stringValue\":\"jing\",\"willContinue\":true}],\"willContinue\":true}}]}}]}\n\n\
                     data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"functionCall\":{\"id\":\"call-1\",\"name\":\"lookup\",\"args\":{\"city\":\"Beijing\"},\"willContinue\":false}}]}}]}\n\n\
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

    let mut done_response = None;
    while let Some(event) = stream.next_event().await.unwrap() {
        if let GenerateContentStreamEvent::Done(response) = event {
            done_response = Some(response);
            break;
        }
    }

    let response = done_response.unwrap();
    let calls = response.function_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id.as_deref(), Some("call-1"));
    assert_eq!(calls[0].name.as_deref(), Some("lookup"));
    assert_eq!(calls[0].args, Some(json!({"city": "Beijing"})));
    assert!(calls[0].partial_args.is_none());
    assert_eq!(calls[0].will_continue, Some(false));
}

#[test]
fn test_merge_stream_response_preserves_sparse_candidates_without_index() {
    let mut aggregate = None;
    merge_stream_response(
        &mut aggregate,
        &GenerateContentResponse {
            sdk_http_response: None,
            candidates: vec![
                Candidate {
                    content: Some(Content::from_parts(vec![Part::text("first")], Role::Model)),
                    citation_metadata: None,
                    finish_message: None,
                    token_count: None,
                    finish_reason: None,
                    avg_logprobs: None,
                    grounding_metadata: None,
                    index: None,
                    logprobs_result: None,
                    safety_ratings: Vec::new(),
                    url_context_metadata: None,
                },
                Candidate {
                    content: Some(Content::from_parts(vec![Part::text("second")], Role::Model)),
                    citation_metadata: None,
                    finish_message: None,
                    token_count: None,
                    finish_reason: None,
                    avg_logprobs: None,
                    grounding_metadata: None,
                    index: None,
                    logprobs_result: None,
                    safety_ratings: Vec::new(),
                    url_context_metadata: None,
                },
            ],
            create_time: None,
            automatic_function_calling_history: None,
            prompt_feedback: None,
            usage_metadata: None,
            model_version: None,
            response_id: None,
        },
    );

    merge_stream_response(
        &mut aggregate,
        &GenerateContentResponse {
            sdk_http_response: None,
            candidates: vec![Candidate {
                content: Some(Content::from_parts(
                    vec![Part::function_call(FunctionCall {
                        id: None,
                        name: Some("lookup".into()),
                        args: None,
                        partial_args: Some(vec![PartialArg {
                            null_value: None,
                            number_value: None,
                            string_value: Some("fragment".into()),
                            bool_value: None,
                            json_path: Some("$.city".into()),
                            will_continue: Some(true),
                        }]),
                        will_continue: Some(true),
                    })],
                    Role::Model,
                )),
                citation_metadata: None,
                finish_message: None,
                token_count: None,
                finish_reason: None,
                avg_logprobs: None,
                grounding_metadata: None,
                index: None,
                logprobs_result: None,
                safety_ratings: Vec::new(),
                url_context_metadata: None,
            }],
            create_time: None,
            automatic_function_calling_history: None,
            prompt_feedback: None,
            usage_metadata: None,
            model_version: None,
            response_id: None,
        },
    );

    let aggregate = aggregate.unwrap();
    assert_eq!(aggregate.candidates.len(), 3);
    assert_eq!(
        aggregate.candidates[0]
            .content
            .as_ref()
            .unwrap()
            .first_text(),
        Some("first")
    );
    assert_eq!(
        aggregate.candidates[1]
            .content
            .as_ref()
            .unwrap()
            .first_text(),
        Some("second")
    );
    assert_eq!(aggregate.function_calls().len(), 1);
}

#[test]
fn test_merge_stream_response_merges_indexed_candidate_metadata() {
    let mut aggregate = Some(GenerateContentResponse {
        sdk_http_response: None,
        candidates: vec![
            Candidate {
                content: Some(Content {
                    role: None,
                    parts: vec![Part::text("first")],
                }),
                citation_metadata: None,
                finish_message: None,
                token_count: None,
                finish_reason: None,
                avg_logprobs: None,
                grounding_metadata: None,
                index: Some(7),
                logprobs_result: None,
                safety_ratings: Vec::new(),
                url_context_metadata: None,
            },
            Candidate {
                content: None,
                citation_metadata: None,
                finish_message: None,
                token_count: None,
                finish_reason: None,
                avg_logprobs: None,
                grounding_metadata: None,
                index: Some(8),
                logprobs_result: None,
                safety_ratings: Vec::new(),
                url_context_metadata: None,
            },
        ],
        create_time: None,
        automatic_function_calling_history: None,
        prompt_feedback: None,
        usage_metadata: None,
        model_version: None,
        response_id: None,
    });

    merge_stream_response(
        &mut aggregate,
        &GenerateContentResponse {
            sdk_http_response: Some(HttpResponse {
                headers: None,
                body: Some("{\"ok\":true}".into()),
            }),
            candidates: vec![
                Candidate {
                    content: Some(Content::from_parts(
                        vec![Part::text(" second")],
                        Role::Model,
                    )),
                    citation_metadata: Some(CitationMetadata {
                        citations: Some(vec![Citation {
                            end_index: Some(2),
                            license: None,
                            publication_date: None,
                            start_index: Some(0),
                            title: Some("cite".into()),
                            uri: Some("https://example.com/cite".into()),
                        }]),
                    }),
                    finish_message: Some("done".into()),
                    token_count: Some(4),
                    finish_reason: Some(FinishReason::Stop),
                    avg_logprobs: Some(0.25),
                    grounding_metadata: Some(GroundingMetadata::default()),
                    index: Some(7),
                    logprobs_result: Some(LogprobsResult {
                        top_candidates: vec![TopCandidates {
                            candidates: vec![LogprobCandidate {
                                token: "first".into(),
                                token_id: 1,
                                log_probability: -0.1,
                            }],
                        }],
                        chosen_candidates: Vec::new(),
                        log_probability_sum: Some(-0.1),
                    }),
                    safety_ratings: vec![SafetyRating {
                        category: HarmCategory::HarmCategoryHarassment,
                        probability: HarmProbability::Low,
                        blocked: Some(false),
                        overwritten_threshold: None,
                        probability_score: None,
                        severity: None,
                        severity_score: None,
                    }],
                    url_context_metadata: Some(UrlContextMetadata {
                        url_metadata: Some(vec![UrlMetadata {
                            retrieved_url: Some("https://example.com".into()),
                            url_retrieval_status: Some(
                                UrlRetrievalStatus::UrlRetrievalStatusSuccess,
                            ),
                        }]),
                    }),
                },
                Candidate {
                    content: Some(Content::from_parts(vec![Part::text("fresh")], Role::Model)),
                    citation_metadata: None,
                    finish_message: None,
                    token_count: None,
                    finish_reason: None,
                    avg_logprobs: None,
                    grounding_metadata: None,
                    index: Some(8),
                    logprobs_result: None,
                    safety_ratings: Vec::new(),
                    url_context_metadata: None,
                },
            ],
            create_time: Some("2026-04-20T12:00:00Z".into()),
            automatic_function_calling_history: Some(vec![Content::model("history")]),
            prompt_feedback: Some(PromptFeedback {
                block_reason: Some(BlockedReason::Other),
                block_reason_message: Some("blocked".into()),
                safety_ratings: Vec::new(),
            }),
            usage_metadata: Some(GenerateContentResponseUsageMetadata {
                cache_tokens_details: None,
                cached_content_token_count: None,
                candidates_token_count: None,
                candidates_tokens_details: None,
                prompt_token_count: None,
                prompt_tokens_details: None,
                thoughts_token_count: None,
                tool_use_prompt_token_count: None,
                tool_use_prompt_tokens_details: None,
                total_token_count: Some(9),
                traffic_type: None,
            }),
            model_version: Some("v2".into()),
            response_id: Some("resp-1".into()),
        },
    );

    let aggregate = aggregate.unwrap();
    assert_eq!(
        aggregate
            .sdk_http_response
            .as_ref()
            .unwrap()
            .body
            .as_deref(),
        Some("{\"ok\":true}")
    );
    assert_eq!(
        aggregate.create_time.as_deref(),
        Some("2026-04-20T12:00:00Z")
    );
    assert_eq!(
        aggregate
            .automatic_function_calling_history
            .as_ref()
            .unwrap()[0]
            .first_text(),
        Some("history")
    );
    assert_eq!(
        aggregate.prompt_feedback.as_ref().unwrap().block_reason,
        Some(BlockedReason::Other)
    );
    assert_eq!(
        aggregate.usage_metadata.as_ref().unwrap().total_token_count,
        Some(9)
    );
    assert_eq!(aggregate.model_version.as_deref(), Some("v2"));
    assert_eq!(aggregate.response_id.as_deref(), Some("resp-1"));

    let merged = &aggregate.candidates[0];
    assert_eq!(merged.content.as_ref().unwrap().role, Some(Role::Model));
    assert_eq!(
        merged.content.as_ref().unwrap().first_text(),
        Some("first second")
    );
    assert_eq!(merged.finish_message.as_deref(), Some("done"));
    assert_eq!(merged.token_count, Some(4));
    assert_eq!(merged.finish_reason, Some(FinishReason::Stop));
    assert_eq!(merged.avg_logprobs, Some(0.25));
    assert!(merged.citation_metadata.is_some());
    assert!(merged.grounding_metadata.is_some());
    assert!(merged.logprobs_result.is_some());
    assert_eq!(merged.safety_ratings.len(), 1);
    assert!(merged.url_context_metadata.is_some());

    let filled = &aggregate.candidates[1];
    assert_eq!(filled.content.as_ref().unwrap().first_text(), Some("fresh"));
}

#[test]
fn test_merge_stream_response_merges_single_candidate_when_index_appears_later() {
    let mut aggregate = None;
    merge_stream_response(
        &mut aggregate,
        &GenerateContentResponse {
            sdk_http_response: None,
            candidates: vec![Candidate {
                content: Some(Content::from_parts(vec![Part::text("Hel")], Role::Model)),
                citation_metadata: None,
                finish_message: None,
                token_count: None,
                finish_reason: None,
                avg_logprobs: None,
                grounding_metadata: None,
                index: None,
                logprobs_result: None,
                safety_ratings: Vec::new(),
                url_context_metadata: None,
            }],
            create_time: None,
            automatic_function_calling_history: None,
            prompt_feedback: None,
            usage_metadata: None,
            model_version: None,
            response_id: None,
        },
    );

    merge_stream_response(
        &mut aggregate,
        &GenerateContentResponse {
            sdk_http_response: None,
            candidates: vec![Candidate {
                content: Some(Content::from_parts(vec![Part::text("lo")], Role::Model)),
                citation_metadata: None,
                finish_message: None,
                token_count: None,
                finish_reason: None,
                avg_logprobs: None,
                grounding_metadata: None,
                index: Some(0),
                logprobs_result: None,
                safety_ratings: Vec::new(),
                url_context_metadata: None,
            }],
            create_time: None,
            automatic_function_calling_history: None,
            prompt_feedback: None,
            usage_metadata: None,
            model_version: None,
            response_id: None,
        },
    );

    let aggregate = aggregate.unwrap();
    assert_eq!(aggregate.candidates.len(), 1);
    assert_eq!(aggregate.text().as_deref(), Some("Hello"));
    assert_eq!(aggregate.candidates[0].index, Some(0));
}

#[test]
fn test_stream_merge_helpers_respect_context_and_targets() {
    let resolution_low = PartMediaResolution {
        level: Some(PartMediaResolutionLevel::MediaResolutionLow),
        num_tokens: Some(1),
    };
    let resolution_medium = PartMediaResolution {
        level: Some(PartMediaResolutionLevel::MediaResolutionMedium),
        num_tokens: Some(2),
    };
    assert!(media_resolution_matches(&None, &None));
    assert!(!media_resolution_matches(
        &Some(resolution_low.clone()),
        &Some(resolution_medium.clone())
    ));
    assert!(!media_resolution_matches(
        &Some(resolution_low.clone()),
        &None
    ));

    let video_a = VideoMetadata {
        start_offset: Some("0s".into()),
        end_offset: Some("1s".into()),
        fps: Some(24.0),
    };
    let video_b = VideoMetadata {
        start_offset: Some("0s".into()),
        end_offset: Some("2s".into()),
        fps: Some(24.0),
    };
    assert!(video_metadata_matches(&None, &None));
    assert!(video_metadata_matches(
        &Some(video_a.clone()),
        &Some(video_a.clone())
    ));
    assert!(!video_metadata_matches(
        &Some(video_a.clone()),
        &Some(video_b)
    ));
    assert!(!video_metadata_matches(&Some(video_a.clone()), &None));

    let lookup_call = FunctionCall {
        id: Some("call-1".into()),
        name: Some("lookup".into()),
        args: None,
        partial_args: Some(vec![PartialArg {
            null_value: None,
            number_value: None,
            string_value: Some("Bei".into()),
            bool_value: None,
            json_path: Some("$.city".into()),
            will_continue: Some(true),
        }]),
        will_continue: Some(true),
    };
    let search_call = FunctionCall {
        id: None,
        name: Some("search".into()),
        args: None,
        partial_args: None,
        will_continue: None,
    };
    assert!(!function_calls_share_target(
        &lookup_call,
        &FunctionCall {
            id: Some("call-2".into()),
            ..lookup_call.clone()
        }
    ));
    assert!(!function_calls_share_target(
        &search_call,
        &FunctionCall {
            id: None,
            name: Some("lookup".into()),
            args: None,
            partial_args: None,
            will_continue: None,
        }
    ));
    assert!(function_calls_share_target(
        &search_call,
        &FunctionCall {
            id: None,
            name: Some("search".into()),
            args: None,
            partial_args: None,
            will_continue: None,
        }
    ));

    let mut merged_call_part = Part::function_call(lookup_call);
    assert!(merge_stream_part(
        Some(&mut merged_call_part),
        &Part::function_call(FunctionCall {
            id: Some("call-1".into()),
            name: Some("lookup".into()),
            args: None,
            partial_args: Some(vec![PartialArg {
                null_value: None,
                number_value: None,
                string_value: Some("jing".into()),
                bool_value: None,
                json_path: Some("$.city".into()),
                will_continue: Some(true),
            }]),
            will_continue: Some(true),
        })
    ));
    let merged_call = merged_call_part.function_call_ref().unwrap();
    assert_eq!(merged_call.partial_args.as_ref().unwrap().len(), 2);
    assert_eq!(merged_call.id.as_deref(), Some("call-1"));
    assert_eq!(merged_call.name.as_deref(), Some("lookup"));
    assert_eq!(merged_call.will_continue, Some(true));

    let mut finalized_call = FunctionCall {
        id: None,
        name: None,
        args: None,
        partial_args: Some(vec![PartialArg {
            null_value: None,
            number_value: None,
            string_value: Some("stale".into()),
            bool_value: None,
            json_path: Some("$.city".into()),
            will_continue: Some(true),
        }]),
        will_continue: None,
    };
    merge_function_call(
        &mut finalized_call,
        &FunctionCall {
            id: Some("call-9".into()),
            name: Some("lookup".into()),
            args: Some(json!({"city": "Beijing"})),
            partial_args: None,
            will_continue: Some(false),
        },
    );
    assert_eq!(finalized_call.id.as_deref(), Some("call-9"));
    assert_eq!(finalized_call.name.as_deref(), Some("lookup"));
    assert_eq!(finalized_call.args, Some(json!({"city": "Beijing"})));
    assert!(finalized_call.partial_args.is_none());
    assert_eq!(finalized_call.will_continue, Some(false));

    let mut text_part = Part::text("hello");
    assert!(!merge_stream_part(
        Some(&mut text_part),
        &Part::function_call(FunctionCall {
            id: Some("call-9".into()),
            name: Some("lookup".into()),
            args: None,
            partial_args: None,
            will_continue: None,
        })
    ));
    assert!(!merge_stream_part(None, &Part::text("next")));

    let mut thought_part = Part::text("a").with_thought(true);
    assert!(!merge_stream_part(
        Some(&mut thought_part),
        &Part::text("b")
    ));

    let mut resolution_part = Part::text("a").with_media_resolution(resolution_low);
    assert!(!merge_stream_part(
        Some(&mut resolution_part),
        &Part::text("b").with_media_resolution(resolution_medium)
    ));

    let mut parts = vec![Part::text("a").with_video_metadata(video_a)];
    merge_content_parts(
        &mut parts,
        &[Part::text("b").with_video_metadata(VideoMetadata {
            start_offset: Some("0s".into()),
            end_offset: Some("3s".into()),
            fps: Some(24.0),
        })],
    );
    assert_eq!(parts.len(), 2);
}

#[test]
fn test_merge_content_parts_uses_part_positions_for_multi_part_chunks() {
    let mut existing_parts = vec![
        Part::text("hello"),
        Part::function_call(FunctionCall {
            id: Some("call-1".into()),
            name: Some("lookup".into()),
            args: None,
            partial_args: Some(vec![PartialArg {
                null_value: None,
                number_value: None,
                string_value: Some("Bei".into()),
                bool_value: None,
                json_path: Some("$.city".into()),
                will_continue: Some(true),
            }]),
            will_continue: Some(true),
        }),
    ];

    merge_content_parts(
        &mut existing_parts,
        &[
            Part::text(" world"),
            Part::function_call(FunctionCall {
                id: Some("call-1".into()),
                name: Some("lookup".into()),
                args: None,
                partial_args: Some(vec![PartialArg {
                    null_value: None,
                    number_value: None,
                    string_value: Some("jing".into()),
                    bool_value: None,
                    json_path: Some("$.city".into()),
                    will_continue: Some(true),
                }]),
                will_continue: Some(true),
            }),
        ],
    );

    assert_eq!(existing_parts.len(), 2);
    assert_eq!(existing_parts[0].text_value(), Some("hello world"));
    let call = existing_parts[1].function_call_ref().unwrap();
    assert_eq!(call.id.as_deref(), Some("call-1"));
    assert_eq!(call.name.as_deref(), Some("lookup"));
    assert_eq!(call.partial_args.as_ref().unwrap().len(), 2);
}

#[test]
fn test_merge_content_parts_merges_sparse_function_call_delta() {
    let mut existing_parts = vec![
        Part::text("prefix"),
        Part::function_call(FunctionCall {
            id: Some("call-1".into()),
            name: Some("lookup".into()),
            args: None,
            partial_args: Some(vec![PartialArg {
                null_value: None,
                number_value: None,
                string_value: Some("Bei".into()),
                bool_value: None,
                json_path: Some("$.city".into()),
                will_continue: Some(true),
            }]),
            will_continue: Some(true),
        }),
    ];

    merge_content_parts(
        &mut existing_parts,
        &[Part::function_call(FunctionCall {
            id: Some("call-1".into()),
            name: Some("lookup".into()),
            args: Some(json!({"city": "Beijing"})),
            partial_args: None,
            will_continue: Some(false),
        })],
    );

    assert_eq!(existing_parts.len(), 2);
    let call = existing_parts[1].function_call_ref().unwrap();
    assert_eq!(call.id.as_deref(), Some("call-1"));
    assert_eq!(call.name.as_deref(), Some("lookup"));
    assert_eq!(call.args, Some(json!({"city": "Beijing"})));
    assert!(call.partial_args.is_none());
    assert_eq!(call.will_continue, Some(false));
}

#[test]
fn test_find_mergeable_part_index_requires_unique_sparse_match() {
    let existing_parts = vec![
        Part::function_call(FunctionCall {
            id: Some("call-0".into()),
            name: Some("seed".into()),
            args: None,
            partial_args: None,
            will_continue: None,
        }),
        Part::text("a"),
        Part::text("b"),
    ];
    assert_eq!(
        find_mergeable_part_index(&existing_parts, 0, &Part::text("c")),
        None
    );

    let mixed_parts = vec![
        Part::text("a"),
        Part::function_call(FunctionCall {
            id: Some("call-1".into()),
            name: Some("lookup".into()),
            args: None,
            partial_args: None,
            will_continue: None,
        }),
    ];
    assert_eq!(
        find_mergeable_part_index(
            &mixed_parts,
            0,
            &Part::function_call(FunctionCall {
                id: Some("call-1".into()),
                name: Some("lookup".into()),
                args: None,
                partial_args: None,
                will_continue: None,
            }),
        ),
        Some(1)
    );
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
