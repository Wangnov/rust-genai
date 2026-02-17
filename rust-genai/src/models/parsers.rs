use crate::client::Backend;
use crate::error::Result;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use rust_genai_types::models::{
    ContentEmbedding, EditImageResponse, EmbedContentMetadata, EmbedContentResponse, EntityLabel,
    GenerateImagesResponse, GenerateVideosOperation, GenerateVideosResponse, GeneratedImage,
    GeneratedImageMask, GeneratedVideo, Image, RecontextImageResponse, SafetyAttributes,
    SegmentImageResponse, UpscaleImageResponse, Video,
};
use rust_genai_types::operations::OperationError;
use serde_json::Value;

pub(super) fn convert_vertex_embed_response(value: &Value) -> Result<EmbedContentResponse> {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut embeddings: Vec<ContentEmbedding> = Vec::new();
    for item in predictions {
        if let Some(embedding_value) = item.get("embeddings") {
            let embedding: ContentEmbedding = serde_json::from_value(embedding_value.clone())?;
            embeddings.push(embedding);
        }
    }

    let metadata: Option<EmbedContentMetadata> = value
        .get("metadata")
        .map(|meta| serde_json::from_value(meta.clone()))
        .transpose()?;

    Ok(EmbedContentResponse {
        sdk_http_response: None,
        embeddings: Some(embeddings),
        metadata,
    })
}

pub(super) fn parse_generate_images_response(value: &Value) -> GenerateImagesResponse {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_images = Vec::new();
    for item in predictions {
        generated_images.push(parse_generated_image(&item));
    }

    let positive_prompt_safety_attributes = value
        .get("positivePromptSafetyAttributes")
        .and_then(parse_safety_attributes);

    GenerateImagesResponse {
        sdk_http_response: None,
        generated_images,
        positive_prompt_safety_attributes,
    }
}

pub(super) fn parse_edit_image_response(value: &Value) -> EditImageResponse {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_images = Vec::new();
    for item in predictions {
        generated_images.push(parse_generated_image(&item));
    }

    EditImageResponse {
        sdk_http_response: None,
        generated_images,
    }
}

pub(super) fn parse_upscale_image_response(value: &Value) -> UpscaleImageResponse {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_images = Vec::new();
    for item in predictions {
        generated_images.push(parse_generated_image(&item));
    }

    UpscaleImageResponse {
        sdk_http_response: None,
        generated_images,
    }
}

pub(super) fn parse_recontext_image_response(value: &Value) -> RecontextImageResponse {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_images = Vec::new();
    for item in predictions {
        generated_images.push(parse_generated_image(&item));
    }

    RecontextImageResponse { generated_images }
}

pub(super) fn parse_segment_image_response(value: &Value) -> SegmentImageResponse {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_masks = Vec::new();
    for item in predictions {
        generated_masks.push(parse_generated_image_mask(&item));
    }

    SegmentImageResponse { generated_masks }
}

pub(super) fn parse_generate_videos_operation(
    value: Value,
    backend: Backend,
) -> Result<GenerateVideosOperation> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RawOperation {
        name: Option<String>,
        metadata: Option<Value>,
        done: Option<bool>,
        error: Option<OperationError>,
        response: Option<Value>,
    }

    let mut raw: RawOperation = serde_json::from_value(value)?;

    let response = match backend {
        Backend::GeminiApi => raw
            .response
            .take()
            .and_then(|response| response.get("generateVideoResponse").cloned().or(Some(response))),
        Backend::VertexAi => raw.response.take(),
    };

    let response = response
        .map(|value| parse_generate_videos_response(&value, backend))
        .transpose()?;

    Ok(GenerateVideosOperation {
        name: raw.name,
        metadata: raw.metadata,
        done: raw.done,
        error: raw.error,
        response,
    })
}

fn parse_generate_videos_response(value: &Value, backend: Backend) -> Result<GenerateVideosResponse> {
    let mut generated_videos = Vec::new();

    match backend {
        Backend::GeminiApi => {
            // Gemini API uses `generatedSamples` and wraps each item with `video`.
            if let Some(items) = value.get("generatedSamples").and_then(Value::as_array) {
                for item in items {
                    let video_value = item.get("video").unwrap_or(item);
                    generated_videos.push(GeneratedVideo {
                        video: parse_video(video_value, backend)?,
                    });
                }
            }
        }
        Backend::VertexAi => {
            // Vertex AI uses `videos` and each item is a Video object.
            if let Some(items) = value.get("videos").and_then(Value::as_array) {
                for item in items {
                    let video_value = item.get("_self").unwrap_or(item);
                    generated_videos.push(GeneratedVideo {
                        video: parse_video(video_value, backend)?,
                    });
                }
            }
        }
    }

    let rai_media_filtered_count = value
        .get("raiMediaFilteredCount")
        .and_then(Value::as_i64)
        .and_then(|v| i32::try_from(v).ok());
    let rai_media_filtered_reasons = value
        .get("raiMediaFilteredReasons")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        });

    Ok(GenerateVideosResponse {
        generated_videos,
        rai_media_filtered_count,
        rai_media_filtered_reasons,
    })
}

fn parse_video(value: &Value, backend: Backend) -> Result<Option<Video>> {
    let obj = match value.as_object() {
        Some(obj) => obj,
        None => return Ok(None),
    };

    let uri_key = match backend {
        Backend::GeminiApi => "uri",
        Backend::VertexAi => "gcsUri",
    };
    let bytes_key = match backend {
        Backend::GeminiApi => "encodedVideo",
        Backend::VertexAi => "bytesBase64Encoded",
    };
    let mime_key = match backend {
        Backend::GeminiApi => "encoding",
        Backend::VertexAi => "mimeType",
    };

    let uri = obj
        .get(uri_key)
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let video_bytes = match obj.get(bytes_key).and_then(Value::as_str) {
        Some(encoded) => Some(STANDARD.decode(encoded.as_bytes()).map_err(|e| {
            crate::error::Error::Parse {
                message: format!("Invalid base64 in video bytes: {e}"),
            }
        })?),
        None => None,
    };
    let mime_type = obj
        .get(mime_key)
        .and_then(Value::as_str)
        .map(ToString::to_string);

    if uri.is_none() && video_bytes.is_none() && mime_type.is_none() {
        return Ok(None);
    }

    Ok(Some(Video {
        uri,
        video_bytes,
        mime_type,
    }))
}

fn parse_generated_image(value: &Value) -> GeneratedImage {
    let image = serde_json::from_value::<Image>(value.clone()).ok();

    let rai_filtered_reason = value
        .get("raiFilteredReason")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);
    let enhanced_prompt = value
        .get("enhancedPrompt")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);

    let safety_attributes = parse_safety_attributes(value);

    GeneratedImage {
        image,
        rai_filtered_reason,
        safety_attributes,
        enhanced_prompt,
    }
}

pub(super) fn parse_generated_image_mask(value: &Value) -> GeneratedImageMask {
    let mask = serde_json::from_value::<Image>(value.clone()).ok();
    let labels = value
        .get("labels")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(parse_entity_label)
                .collect::<Vec<EntityLabel>>()
        });

    GeneratedImageMask { mask, labels }
}

fn parse_entity_label(value: &Value) -> Option<EntityLabel> {
    let obj = value.as_object()?;
    let label = obj
        .get("label")
        .and_then(|value| value.as_str())
        .map(ToString::to_string);
    let score = obj.get("score").and_then(|value| match value {
        Value::Number(num) => num.to_string().parse::<f32>().ok(),
        Value::String(text) => text.parse::<f32>().ok(),
        _ => None,
    });

    Some(EntityLabel { label, score })
}

pub(super) fn parse_safety_attributes(value: &Value) -> Option<SafetyAttributes> {
    let obj = value.as_object()?;
    let safety = obj
        .get("safetyAttributes")
        .and_then(serde_json::Value::as_object);

    let categories = obj
        .get("categories")
        .or_else(|| safety.and_then(|s| s.get("categories")))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        });

    let scores = obj
        .get("scores")
        .or_else(|| safety.and_then(|s| s.get("scores")))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    Value::Number(num) => num.to_string().parse::<f32>().ok(),
                    Value::String(text) => text.parse::<f32>().ok(),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });

    let content_type = obj
        .get("contentType")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);

    if categories.is_none() && scores.is_none() && content_type.is_none() {
        None
    } else {
        Some(SafetyAttributes {
            categories,
            scores,
            content_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Backend;
    use serde_json::json;

    #[test]
    fn test_parse_image_responses_and_safety() {
        let image_value = json!({
            "bytesBase64Encoded": "AQID",
            "mimeType": "image/png",
            "raiFilteredReason": "FILTERED",
            "enhancedPrompt": "better",
            "safetyAttributes": {
                "categories": ["SAFE"],
                "scores": [0.1],
                "contentType": "IMAGE"
            }
        });
        let response = parse_generate_images_response(&json!({
            "predictions": [&image_value],
            "positivePromptSafetyAttributes": {
                "categories": ["SAFE"],
                "scores": [0.2],
                "contentType": "TEXT"
            }
        }));
        assert_eq!(response.generated_images.len(), 1);
        assert!(response.positive_prompt_safety_attributes.is_some());

        let edit = parse_edit_image_response(&json!({"predictions": [&image_value]}));
        assert_eq!(edit.generated_images.len(), 1);

        let upscale = parse_upscale_image_response(&json!({"predictions": [image_value]}));
        assert_eq!(upscale.generated_images.len(), 1);
    }

    #[test]
    fn test_parse_generated_image_mask_and_operation() {
        let mask = parse_generated_image_mask(&json!({
            "bytesBase64Encoded": "AQID",
            "mimeType": "image/png",
            "labels": [
                {"label": "cat", "score": "0.9"},
                {"label": "dog", "score": 0.2}
            ]
        }));
        assert!(mask.mask.is_some());
        assert_eq!(mask.labels.as_ref().unwrap().len(), 2);

        let op = parse_generate_videos_operation(
            json!({
                "name": "operations/1",
                "response": {
                    "generateVideoResponse": {
                        "generatedSamples": [
                            {
                                "video": {
                                    "uri": "https://example.com/v.mp4",
                                    "encoding": "video/mp4",
                                    "encodedVideo": "AQID"
                                }
                            }
                        ]
                    }
                }
            }),
            Backend::GeminiApi,
        )
        .unwrap();
        let resp = op.response.unwrap();
        assert_eq!(resp.generated_videos.len(), 1);
        let video = resp.generated_videos[0].video.as_ref().unwrap();
        assert_eq!(video.uri.as_deref(), Some("https://example.com/v.mp4"));
        assert_eq!(video.mime_type.as_deref(), Some("video/mp4"));
        assert_eq!(video.video_bytes.as_deref(), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn test_vertex_embed_response_parsing() {
        let response = convert_vertex_embed_response(&json!({
            "predictions": [
                {"embeddings": {"values": [0.1, 0.2], "statistics": {"tokenCount": 2}}}
            ],
            "metadata": {"billableCharacterCount": 10}
        }))
        .unwrap();
        assert_eq!(response.embeddings.as_ref().unwrap().len(), 1);
        assert!(response.metadata.is_some());
    }

    #[test]
    fn test_parse_recontext_and_segment_responses() {
        let value =
            json!({"predictions": [ {"bytesBase64Encoded": "AQID", "mimeType": "image/png"} ]});
        let recontext = parse_recontext_image_response(&value);
        assert_eq!(recontext.generated_images.len(), 1);
        let segment = parse_segment_image_response(&json!({
            "predictions": [ {"bytesBase64Encoded": "AQID", "mimeType": "image/png"} ]
        }));
        assert_eq!(segment.generated_masks.len(), 1);
    }

    #[test]
    fn test_parse_safety_attributes_variants() {
        let value = json!({
            "categories": ["SAFE"],
            "scores": [0.1],
            "contentType": "TEXT"
        });
        let safety = parse_safety_attributes(&value).unwrap();
        assert_eq!(safety.categories.unwrap().len(), 1);
        assert_eq!(safety.content_type.as_deref(), Some("TEXT"));
    }
}
