use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde_json::{Map, Number, Value};

use crate::client::Backend;
use crate::error::{Error, Result};
use rust_genai_types::models::{
    Image, ReferenceImage, Video, VideoGenerationMask, VideoGenerationReferenceImage,
};

pub(super) fn image_to_mldev(image: &Image) -> Result<Value> {
    if image.gcs_uri.is_some() {
        return Err(Error::InvalidConfig {
            message: "gcs_uri is not supported in Gemini API".into(),
        });
    }
    let mut map = Map::new();
    if let Some(bytes) = &image.image_bytes {
        map.insert(
            "bytesBase64Encoded".to_string(),
            Value::String(STANDARD.encode(bytes)),
        );
    }
    if let Some(mime) = &image.mime_type {
        map.insert("mimeType".to_string(), Value::String(mime.clone()));
    }
    Ok(Value::Object(map))
}

pub(super) fn image_to_vertex(image: &Image) -> Value {
    let mut map = Map::new();
    if let Some(gcs_uri) = &image.gcs_uri {
        map.insert("gcsUri".to_string(), Value::String(gcs_uri.clone()));
    }
    if let Some(bytes) = &image.image_bytes {
        map.insert(
            "bytesBase64Encoded".to_string(),
            Value::String(STANDARD.encode(bytes)),
        );
    }
    if let Some(mime) = &image.mime_type {
        map.insert("mimeType".to_string(), Value::String(mime.clone()));
    }
    Value::Object(map)
}

pub(super) fn video_to_mldev(video: &Video) -> Value {
    if let Some(uri) = &video.uri {
        let mut map = Map::new();
        map.insert("uri".to_string(), Value::String(uri.clone()));
        if let Some(bytes) = &video.video_bytes {
            map.insert(
                "encodedVideo".to_string(),
                Value::String(STANDARD.encode(bytes)),
            );
        }
        if let Some(mime) = &video.mime_type {
            map.insert("encoding".to_string(), Value::String(mime.clone()));
        }
        return Value::Object(map);
    }

    let mut map = Map::new();
    if let Some(bytes) = &video.video_bytes {
        map.insert(
            "encodedVideo".to_string(),
            Value::String(STANDARD.encode(bytes)),
        );
    }
    if let Some(mime) = &video.mime_type {
        map.insert("encoding".to_string(), Value::String(mime.clone()));
    }
    Value::Object(map)
}

pub(super) fn video_to_vertex(video: &Video) -> Value {
    let mut map = Map::new();
    if let Some(uri) = &video.uri {
        map.insert("gcsUri".to_string(), Value::String(uri.clone()));
    }
    if let Some(bytes) = &video.video_bytes {
        map.insert(
            "bytesBase64Encoded".to_string(),
            Value::String(STANDARD.encode(bytes)),
        );
    }
    if let Some(mime) = &video.mime_type {
        map.insert("mimeType".to_string(), Value::String(mime.clone()));
    }
    Value::Object(map)
}

pub(super) fn reference_image_to_vertex(image: &ReferenceImage) -> Result<Value> {
    let mut map = Map::new();
    if let Some(reference_image) = &image.reference_image {
        map.insert(
            "referenceImage".to_string(),
            image_to_vertex(reference_image),
        );
    }
    if let Some(reference_id) = image.reference_id {
        map.insert(
            "referenceId".to_string(),
            Value::Number(Number::from(reference_id)),
        );
    }
    if let Some(reference_type) = image.reference_type {
        map.insert(
            "referenceType".to_string(),
            serde_json::to_value(reference_type)?,
        );
    }
    if let Some(config) = &image.mask_image_config {
        map.insert("maskImageConfig".to_string(), serde_json::to_value(config)?);
    }
    if let Some(config) = &image.control_image_config {
        map.insert(
            "controlImageConfig".to_string(),
            serde_json::to_value(config)?,
        );
    }
    if let Some(config) = &image.style_image_config {
        map.insert(
            "styleImageConfig".to_string(),
            serde_json::to_value(config)?,
        );
    }
    if let Some(config) = &image.subject_image_config {
        map.insert(
            "subjectImageConfig".to_string(),
            serde_json::to_value(config)?,
        );
    }
    Ok(Value::Object(map))
}

pub(super) fn video_reference_image_to_value(
    backend: Backend,
    reference: &VideoGenerationReferenceImage,
) -> Result<Value> {
    let mut map = Map::new();
    if let Some(image) = &reference.image {
        let value = match backend {
            Backend::GeminiApi => image_to_mldev(image)?,
            Backend::VertexAi => image_to_vertex(image),
        };
        map.insert("image".to_string(), value);
    }
    if let Some(reference_type) = reference.reference_type {
        map.insert(
            "referenceType".to_string(),
            serde_json::to_value(reference_type)?,
        );
    }
    Ok(Value::Object(map))
}

pub(super) fn video_mask_to_vertex(mask: &VideoGenerationMask) -> Result<Value> {
    let mut map = Map::new();
    if let Some(image) = &mask.image {
        map.insert("image".to_string(), image_to_vertex(image));
    }
    if let Some(mode) = mask.mask_mode {
        map.insert("maskMode".to_string(), serde_json::to_value(mode)?);
    }
    Ok(Value::Object(map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Backend;
    use crate::error::Error;
    use rust_genai_types::enums::{
        ReferenceImageType, VideoGenerationMaskMode, VideoGenerationReferenceType,
    };
    use rust_genai_types::models::{
        ControlReferenceConfig, Image, MaskReferenceConfig, ReferenceImage, StyleReferenceConfig,
        SubjectReferenceConfig, Video, VideoGenerationMask, VideoGenerationReferenceImage,
    };

    #[test]
    fn test_media_converters_basic() {
        let image = Image {
            gcs_uri: Some("gs://img.png".to_string()),
            image_bytes: Some(vec![1, 2]),
            mime_type: Some("image/png".to_string()),
        };
        let vertex_image = image_to_vertex(&image);
        assert!(vertex_image.get("gcsUri").is_some());

        let err = image_to_mldev(&image).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let video = Video {
            uri: Some("gs://video.mp4".to_string()),
            video_bytes: Some(vec![9, 9]),
            mime_type: Some("video/mp4".to_string()),
        };
        let mldev_video = video_to_mldev(&video);
        assert!(mldev_video.get("uri").is_some());
    }

    #[test]
    fn test_reference_and_mask_helpers() {
        let reference = ReferenceImage {
            reference_image: Some(Image {
                gcs_uri: Some("gs://ref.png".to_string()),
                ..Default::default()
            }),
            reference_id: Some(3),
            reference_type: Some(ReferenceImageType::ReferenceTypeStyle),
            ..Default::default()
        };
        let value = reference_image_to_vertex(&reference).unwrap();
        assert!(value.get("referenceId").is_some());

        let video_ref = VideoGenerationReferenceImage {
            image: Some(Image {
                gcs_uri: Some("gs://ref.png".to_string()),
                ..Default::default()
            }),
            reference_type: Some(VideoGenerationReferenceType::Asset),
        };
        let value = video_reference_image_to_value(Backend::VertexAi, &video_ref).unwrap();
        assert!(value.get("referenceType").is_some());

        let mask = VideoGenerationMask {
            image: Some(Image {
                gcs_uri: Some("gs://mask.png".to_string()),
                ..Default::default()
            }),
            mask_mode: Some(VideoGenerationMaskMode::Insert),
        };
        let value = video_mask_to_vertex(&mask).unwrap();
        assert!(value.get("maskMode").is_some());
    }

    #[test]
    fn test_models_media_converter_branches() {
        let video = Video {
            video_bytes: Some(vec![1, 2, 3]),
            mime_type: Some("video/mp4".to_string()),
            ..Default::default()
        };
        let mldev = video_to_mldev(&video);
        assert!(mldev.get("encodedVideo").is_some());

        let vertex = video_to_vertex(&video);
        assert!(vertex.get("bytesBase64Encoded").is_some());
        assert!(vertex.get("mimeType").is_some());

        let reference = ReferenceImage {
            reference_image: Some(Image {
                image_bytes: Some(vec![4, 5]),
                mime_type: Some("image/png".to_string()),
                ..Default::default()
            }),
            reference_id: Some(1),
            reference_type: Some(ReferenceImageType::ReferenceTypeStyle),
            mask_image_config: Some(MaskReferenceConfig::default()),
            control_image_config: Some(ControlReferenceConfig::default()),
            style_image_config: Some(StyleReferenceConfig::default()),
            subject_image_config: Some(SubjectReferenceConfig::default()),
        };
        let reference_value = reference_image_to_vertex(&reference).unwrap();
        assert!(reference_value.get("maskImageConfig").is_some());
        assert!(reference_value.get("controlImageConfig").is_some());
        assert!(reference_value.get("styleImageConfig").is_some());
        assert!(reference_value.get("subjectImageConfig").is_some());
    }
}
