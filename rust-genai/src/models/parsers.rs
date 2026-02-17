use crate::client::Backend;
use crate::error::Result;
use rust_genai_types::models::{
    ContentEmbedding, EditImageResponse, EmbedContentMetadata, EmbedContentResponse, EntityLabel,
    GenerateImagesResponse, GeneratedImage, GeneratedImageMask, Image, RecontextImageResponse,
    SafetyAttributes, SegmentImageResponse, UpscaleImageResponse,
};
use rust_genai_types::operations::Operation;
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

pub(super) fn parse_generate_videos_operation(value: Value, backend: Backend) -> Result<Operation> {
    let mut operation: Operation = serde_json::from_value(value)?;
    if backend == Backend::GeminiApi {
        if let Some(response) = operation.response.take() {
            if let Some(inner) = response.get("generateVideoResponse") {
                operation.response = Some(inner.clone());
            } else {
                operation.response = Some(response);
            }
        }
    }
    Ok(operation)
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
                    "generateVideoResponse": {"ok": true}
                }
            }),
            Backend::GeminiApi,
        )
        .unwrap();
        assert_eq!(
            op.response
                .unwrap()
                .get("ok")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
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
