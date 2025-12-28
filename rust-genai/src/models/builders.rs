use crate::client::Backend;
use crate::error::{Error, Result};
use rust_genai_types::content::{Content, FunctionCall, Part, Role};
use rust_genai_types::models::{
    EditImageConfig, EmbedContentConfig, GenerateImagesConfig, GenerateVideosConfig,
    GenerateVideosSource, Image, RecontextImageConfig, RecontextImageSource, ReferenceImage,
    SegmentImageConfig, SegmentImageSource, UpscaleImageConfig,
};
use serde_json::{Map, Number, Value};

use super::http::transform_model_name;
use super::media::{
    image_to_mldev, image_to_vertex, reference_image_to_vertex, video_mask_to_vertex,
    video_reference_image_to_value, video_to_mldev, video_to_vertex,
};

pub(super) fn build_embed_body_gemini(
    model: &str,
    contents: &[Content],
    config: &EmbedContentConfig,
) -> Result<Value> {
    if config.mime_type.is_some() || config.auto_truncate.is_some() {
        return Err(Error::InvalidConfig {
            message: "mime_type/auto_truncate not supported in Gemini API".into(),
        });
    }

    let mut requests: Vec<Value> = Vec::new();
    for content in contents {
        let mut obj = Map::new();
        obj.insert(
            "model".to_string(),
            Value::String(transform_model_name(Backend::GeminiApi, model)),
        );
        obj.insert("content".to_string(), serde_json::to_value(content)?);
        if let Some(task_type) = &config.task_type {
            obj.insert("taskType".to_string(), Value::String(task_type.clone()));
        }
        if let Some(title) = &config.title {
            obj.insert("title".to_string(), Value::String(title.clone()));
        }
        if let Some(output_dimensionality) = config.output_dimensionality {
            obj.insert(
                "outputDimensionality".to_string(),
                Value::Number(Number::from(i64::from(output_dimensionality))),
            );
        }
        requests.push(Value::Object(obj));
    }

    Ok(Value::Object({
        let mut root = Map::new();
        root.insert("requests".to_string(), Value::Array(requests));
        root
    }))
}

pub(super) fn build_embed_body_vertex(
    contents: &[Content],
    config: &EmbedContentConfig,
) -> Result<Value> {
    let mut instances: Vec<Value> = Vec::new();
    for content in contents {
        let mut obj = Map::new();
        obj.insert("content".to_string(), serde_json::to_value(content)?);
        if let Some(task_type) = &config.task_type {
            obj.insert("task_type".to_string(), Value::String(task_type.clone()));
        }
        if let Some(title) = &config.title {
            obj.insert("title".to_string(), Value::String(title.clone()));
        }
        if let Some(mime_type) = &config.mime_type {
            obj.insert("mimeType".to_string(), Value::String(mime_type.clone()));
        }
        instances.push(Value::Object(obj));
    }

    let mut root = Map::new();
    root.insert("instances".to_string(), Value::Array(instances));

    let mut parameters = Map::new();
    if let Some(output_dimensionality) = config.output_dimensionality {
        parameters.insert(
            "outputDimensionality".to_string(),
            Value::Number(Number::from(i64::from(output_dimensionality))),
        );
    }
    if let Some(auto_truncate) = config.auto_truncate {
        parameters.insert("autoTruncate".to_string(), Value::Bool(auto_truncate));
    }
    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

fn ensure_vertex_only(backend: Backend, field: &str) -> Result<()> {
    if backend == Backend::GeminiApi {
        return Err(Error::InvalidConfig {
            message: format!("{field} is not supported in Gemini API"),
        });
    }
    Ok(())
}

fn build_generate_images_common_parameters(
    config: &GenerateImagesConfig,
) -> Result<Map<String, Value>> {
    let mut parameters = Map::new();
    let mut output_options = Map::new();

    if let Some(value) = config.number_of_images {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = &config.aspect_ratio {
        parameters.insert("aspectRatio".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.guidance_scale {
        parameters.insert(
            "guidanceScale".to_string(),
            Value::Number(Number::from_f64(f64::from(value)).unwrap_or_else(|| Number::from(0))),
        );
    }
    if let Some(value) = config.safety_filter_level {
        parameters.insert("safetySetting".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.person_generation {
        parameters.insert("personGeneration".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.include_safety_attributes {
        parameters.insert("includeSafetyAttributes".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.include_rai_reason {
        parameters.insert("includeRaiReason".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.language {
        parameters.insert("language".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = &config.output_mime_type {
        output_options.insert("mimeType".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.output_compression_quality {
        output_options.insert(
            "compressionQuality".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if !output_options.is_empty() {
        parameters.insert("outputOptions".to_string(), Value::Object(output_options));
    }
    if let Some(value) = &config.image_size {
        parameters.insert("sampleImageSize".to_string(), Value::String(value.clone()));
    }

    Ok(parameters)
}

fn apply_generate_images_vertex_only(
    backend: Backend,
    config: &GenerateImagesConfig,
    parameters: &mut Map<String, Value>,
) -> Result<Option<Value>> {
    if let Some(value) = &config.output_gcs_uri {
        ensure_vertex_only(backend, "output_gcs_uri")?;
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.negative_prompt {
        ensure_vertex_only(backend, "negative_prompt")?;
        parameters.insert("negativePrompt".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.seed {
        ensure_vertex_only(backend, "seed")?;
        parameters.insert("seed".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = config.add_watermark {
        ensure_vertex_only(backend, "add_watermark")?;
        parameters.insert("addWatermark".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.enhance_prompt {
        ensure_vertex_only(backend, "enhance_prompt")?;
        parameters.insert("enhancePrompt".to_string(), Value::Bool(value));
    }
    if let Some(labels) = &config.labels {
        ensure_vertex_only(backend, "labels")?;
        return Ok(Some(serde_json::to_value(labels)?));
    }
    Ok(None)
}

pub(super) fn build_generate_images_body(
    backend: Backend,
    prompt: &str,
    config: &GenerateImagesConfig,
) -> Result<Value> {
    let mut instances = Vec::new();
    let mut instance = Map::new();
    instance.insert("prompt".to_string(), Value::String(prompt.to_string()));
    instances.push(Value::Object(instance));

    let mut root = Map::new();
    root.insert("instances".to_string(), Value::Array(instances));

    let mut parameters = build_generate_images_common_parameters(config)?;
    if let Some(labels) = apply_generate_images_vertex_only(backend, config, &mut parameters)? {
        root.insert("labels".to_string(), labels);
    }

    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

pub(super) fn build_edit_image_body(
    prompt: &str,
    reference_images: &[ReferenceImage],
    config: &EditImageConfig,
) -> Result<Value> {
    let mut instances = Vec::new();
    let mut instance = Map::new();
    instance.insert("prompt".to_string(), Value::String(prompt.to_string()));
    if !reference_images.is_empty() {
        let mut refs = Vec::new();
        for image in reference_images {
            refs.push(reference_image_to_vertex(image)?);
        }
        instance.insert("referenceImages".to_string(), Value::Array(refs));
    }
    instances.push(Value::Object(instance));

    let mut root = Map::new();
    root.insert("instances".to_string(), Value::Array(instances));

    let mut parameters = Map::new();
    let mut output_options = Map::new();
    let mut edit_config = Map::new();

    if let Some(value) = &config.output_gcs_uri {
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.negative_prompt {
        parameters.insert("negativePrompt".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.number_of_images {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = &config.aspect_ratio {
        parameters.insert("aspectRatio".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.guidance_scale {
        parameters.insert(
            "guidanceScale".to_string(),
            Value::Number(Number::from_f64(f64::from(value)).unwrap_or_else(|| Number::from(0))),
        );
    }
    if let Some(value) = config.seed {
        parameters.insert("seed".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = config.safety_filter_level {
        parameters.insert("safetySetting".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.person_generation {
        parameters.insert("personGeneration".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.include_safety_attributes {
        parameters.insert("includeSafetyAttributes".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.include_rai_reason {
        parameters.insert("includeRaiReason".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.language {
        parameters.insert("language".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = &config.output_mime_type {
        output_options.insert("mimeType".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.output_compression_quality {
        output_options.insert(
            "compressionQuality".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if !output_options.is_empty() {
        parameters.insert("outputOptions".to_string(), Value::Object(output_options));
    }
    if let Some(value) = config.add_watermark {
        parameters.insert("addWatermark".to_string(), Value::Bool(value));
    }
    if let Some(labels) = &config.labels {
        root.insert("labels".to_string(), serde_json::to_value(labels)?);
    }
    if let Some(value) = config.edit_mode {
        parameters.insert("editMode".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.base_steps {
        edit_config.insert("baseSteps".to_string(), Value::Number(Number::from(value)));
    }
    if !edit_config.is_empty() {
        parameters.insert("editConfig".to_string(), Value::Object(edit_config));
    }

    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

pub(super) fn build_upscale_image_body(
    image: &Image,
    upscale_factor: &str,
    config: &UpscaleImageConfig,
) -> Result<Value> {
    let mut instances = Vec::new();
    let mut instance = Map::new();
    instance.insert("image".to_string(), image_to_vertex(image));
    instances.push(Value::Object(instance));

    let mut root = Map::new();
    root.insert("instances".to_string(), Value::Array(instances));

    let mut parameters = Map::new();
    let mut output_options = Map::new();
    let mut upscale_config = Map::new();

    parameters.insert(
        "mode".to_string(),
        Value::String(config.mode.clone().unwrap_or_else(|| "upscale".to_string())),
    );

    if let Some(value) = config.number_of_images {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    } else {
        parameters.insert("sampleCount".to_string(), Value::Number(Number::from(1)));
    }

    if let Some(value) = &config.output_gcs_uri {
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.safety_filter_level {
        parameters.insert("safetySetting".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.person_generation {
        parameters.insert("personGeneration".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.include_rai_reason {
        parameters.insert("includeRaiReason".to_string(), Value::Bool(value));
    }
    if let Some(value) = &config.output_mime_type {
        output_options.insert("mimeType".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.output_compression_quality {
        output_options.insert(
            "compressionQuality".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if !output_options.is_empty() {
        parameters.insert("outputOptions".to_string(), Value::Object(output_options));
    }
    if let Some(value) = config.enhance_input_image {
        upscale_config.insert("enhanceInputImage".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.image_preservation_factor {
        upscale_config.insert(
            "imagePreservationFactor".to_string(),
            Value::Number(Number::from_f64(f64::from(value)).unwrap_or_else(|| Number::from(0))),
        );
    }
    upscale_config.insert(
        "upscaleFactor".to_string(),
        Value::String(upscale_factor.to_string()),
    );
    parameters.insert("upscaleConfig".to_string(), Value::Object(upscale_config));

    if let Some(labels) = &config.labels {
        root.insert("labels".to_string(), serde_json::to_value(labels)?);
    }

    root.insert("parameters".to_string(), Value::Object(parameters));

    Ok(Value::Object(root))
}

pub(super) fn build_recontext_image_body(
    source: &RecontextImageSource,
    config: &RecontextImageConfig,
) -> Result<Value> {
    let mut instance = Map::new();
    if let Some(prompt) = &source.prompt {
        instance.insert("prompt".to_string(), Value::String(prompt.clone()));
    }
    if let Some(person_image) = &source.person_image {
        let mut person = Map::new();
        person.insert("image".to_string(), image_to_vertex(person_image));
        instance.insert("personImage".to_string(), Value::Object(person));
    }
    if let Some(product_images) = &source.product_images {
        let mut products = Vec::new();
        for item in product_images {
            if let Some(image) = &item.product_image {
                let mut product = Map::new();
                product.insert("image".to_string(), image_to_vertex(image));
                products.push(Value::Object(product));
            }
        }
        if !products.is_empty() {
            instance.insert("productImages".to_string(), Value::Array(products));
        }
    }

    let mut root = Map::new();
    root.insert(
        "instances".to_string(),
        Value::Array(vec![Value::Object(instance)]),
    );

    let mut parameters = Map::new();
    let mut output_options = Map::new();

    if let Some(value) = config.number_of_images {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = config.base_steps {
        parameters.insert("baseSteps".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = &config.output_gcs_uri {
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.seed {
        parameters.insert("seed".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = config.safety_filter_level {
        parameters.insert("safetySetting".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.person_generation {
        parameters.insert("personGeneration".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.add_watermark {
        parameters.insert("addWatermark".to_string(), Value::Bool(value));
    }
    if let Some(value) = &config.output_mime_type {
        output_options.insert("mimeType".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.output_compression_quality {
        output_options.insert(
            "compressionQuality".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if !output_options.is_empty() {
        parameters.insert("outputOptions".to_string(), Value::Object(output_options));
    }
    if let Some(value) = config.enhance_prompt {
        parameters.insert("enhancePrompt".to_string(), Value::Bool(value));
    }
    if let Some(labels) = &config.labels {
        root.insert("labels".to_string(), serde_json::to_value(labels)?);
    }

    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

pub(super) fn build_segment_image_body(
    source: &SegmentImageSource,
    config: &SegmentImageConfig,
) -> Result<Value> {
    let mut instance = Map::new();
    if let Some(prompt) = &source.prompt {
        instance.insert("prompt".to_string(), Value::String(prompt.clone()));
    }
    if let Some(image) = &source.image {
        instance.insert("image".to_string(), image_to_vertex(image));
    }
    if let Some(scribble) = &source.scribble_image {
        if let Some(image) = &scribble.image {
            let mut scribble_map = Map::new();
            scribble_map.insert("image".to_string(), image_to_vertex(image));
            instance.insert("scribble".to_string(), Value::Object(scribble_map));
        }
    }

    let mut root = Map::new();
    root.insert(
        "instances".to_string(),
        Value::Array(vec![Value::Object(instance)]),
    );

    let mut parameters = Map::new();
    if let Some(value) = config.mode {
        parameters.insert("mode".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.max_predictions {
        parameters.insert(
            "maxPredictions".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = config.confidence_threshold {
        parameters.insert(
            "confidenceThreshold".to_string(),
            Value::Number(Number::from_f64(f64::from(value)).unwrap_or_else(|| Number::from(0))),
        );
    }
    if let Some(value) = config.mask_dilation {
        parameters.insert(
            "maskDilation".to_string(),
            Value::Number(Number::from_f64(f64::from(value)).unwrap_or_else(|| Number::from(0))),
        );
    }
    if let Some(value) = config.binary_color_threshold {
        parameters.insert(
            "binaryColorThreshold".to_string(),
            Value::Number(Number::from_f64(f64::from(value)).unwrap_or_else(|| Number::from(0))),
        );
    }
    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    if let Some(labels) = &config.labels {
        root.insert("labels".to_string(), serde_json::to_value(labels)?);
    }

    Ok(Value::Object(root))
}

fn build_generate_videos_instance(
    backend: Backend,
    source: &GenerateVideosSource,
    config: &GenerateVideosConfig,
) -> Result<Map<String, Value>> {
    let mut instance = Map::new();
    if let Some(prompt) = &source.prompt {
        instance.insert("prompt".to_string(), Value::String(prompt.clone()));
    }
    if let Some(image) = &source.image {
        let value = match backend {
            Backend::GeminiApi => image_to_mldev(image)?,
            Backend::VertexAi => image_to_vertex(image),
        };
        instance.insert("image".to_string(), value);
    }
    if let Some(video) = &source.video {
        let value = match backend {
            Backend::GeminiApi => video_to_mldev(video),
            Backend::VertexAi => video_to_vertex(video),
        };
        instance.insert("video".to_string(), value);
    }
    if let Some(last_frame) = &config.last_frame {
        let value = match backend {
            Backend::GeminiApi => image_to_mldev(last_frame)?,
            Backend::VertexAi => image_to_vertex(last_frame),
        };
        instance.insert("lastFrame".to_string(), value);
    }
    if let Some(reference_images) = &config.reference_images {
        let mut refs = Vec::new();
        for item in reference_images {
            refs.push(video_reference_image_to_value(backend, item)?);
        }
        instance.insert("referenceImages".to_string(), Value::Array(refs));
    }
    if let Some(mask) = &config.mask {
        ensure_vertex_only(backend, "mask")?;
        instance.insert("mask".to_string(), video_mask_to_vertex(mask)?);
    }
    Ok(instance)
}

fn build_generate_videos_parameters(
    backend: Backend,
    config: &GenerateVideosConfig,
) -> Result<Map<String, Value>> {
    let mut parameters = Map::new();

    if let Some(value) = config.number_of_videos {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = &config.output_gcs_uri {
        ensure_vertex_only(backend, "output_gcs_uri")?;
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.fps {
        ensure_vertex_only(backend, "fps")?;
        parameters.insert("fps".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = config.duration_seconds {
        parameters.insert(
            "durationSeconds".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = config.seed {
        ensure_vertex_only(backend, "seed")?;
        parameters.insert("seed".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = &config.aspect_ratio {
        parameters.insert("aspectRatio".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.resolution {
        parameters.insert("resolution".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.person_generation {
        parameters.insert("personGeneration".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.pubsub_topic {
        ensure_vertex_only(backend, "pubsub_topic")?;
        parameters.insert("pubsubTopic".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.negative_prompt {
        parameters.insert("negativePrompt".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.enhance_prompt {
        parameters.insert("enhancePrompt".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.generate_audio {
        ensure_vertex_only(backend, "generate_audio")?;
        parameters.insert("generateAudio".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.compression_quality {
        ensure_vertex_only(backend, "compression_quality")?;
        parameters.insert(
            "compressionQuality".to_string(),
            serde_json::to_value(value)?,
        );
    }

    Ok(parameters)
}

pub(super) fn build_generate_videos_body(
    backend: Backend,
    source: &GenerateVideosSource,
    config: &GenerateVideosConfig,
) -> Result<Value> {
    let instance = build_generate_videos_instance(backend, source, config)?;
    let mut root = Map::new();
    root.insert(
        "instances".to_string(),
        Value::Array(vec![Value::Object(instance)]),
    );

    let parameters = build_generate_videos_parameters(backend, config)?;
    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

pub(super) fn build_function_call_content(function_calls: &[FunctionCall]) -> Content {
    let parts = function_calls
        .iter()
        .cloned()
        .map(Part::function_call)
        .collect();
    Content::from_parts(parts, Role::Model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Backend;
    use crate::error::Error;
    use rust_genai_types::content::Content;
    use rust_genai_types::enums::{
        EditMode, ImagePromptLanguage, PersonGeneration, ReferenceImageType, SafetyFilterLevel,
        SegmentMode, VideoCompressionQuality, VideoGenerationMaskMode,
        VideoGenerationReferenceType,
    };
    use rust_genai_types::models::{
        EditImageConfig, EmbedContentConfig, GenerateImagesConfig, GenerateVideosConfig,
        GenerateVideosSource, Image, ProductImage, RecontextImageConfig, RecontextImageSource,
        ReferenceImage, ScribbleImage, SegmentImageConfig, SegmentImageSource, UpscaleImageConfig,
        Video, VideoGenerationMask, VideoGenerationReferenceImage,
    };

    #[test]
    fn test_build_recontext_image_body() {
        let source = RecontextImageSource {
            prompt: Some("test".to_string()),
            person_image: Some(Image {
                gcs_uri: Some("gs://person.png".to_string()),
                ..Default::default()
            }),
            product_images: Some(vec![ProductImage {
                product_image: Some(Image {
                    gcs_uri: Some("gs://product.png".to_string()),
                    ..Default::default()
                }),
            }]),
        };
        let config = RecontextImageConfig {
            number_of_images: Some(2),
            ..Default::default()
        };

        let body = build_recontext_image_body(&source, &config).unwrap();
        let instances = body.get("instances").and_then(Value::as_array).unwrap();
        let instance = instances[0].as_object().unwrap();
        assert!(instance.get("prompt").is_some());
        assert!(instance.get("personImage").is_some());
        assert!(instance.get("productImages").is_some());
    }

    #[test]
    fn test_build_segment_image_body() {
        let source = SegmentImageSource {
            prompt: Some("foreground".to_string()),
            image: Some(Image {
                gcs_uri: Some("gs://input.png".to_string()),
                ..Default::default()
            }),
            scribble_image: None,
        };
        let config = SegmentImageConfig {
            mode: Some(SegmentMode::Foreground),
            ..Default::default()
        };

        let body = build_segment_image_body(&source, &config).unwrap();
        let instances = body.get("instances").and_then(Value::as_array).unwrap();
        let instance = instances[0].as_object().unwrap();
        assert!(instance.get("image").is_some());
        assert!(body.get("parameters").is_some());
    }

    #[test]
    fn test_build_embed_body_gemini_and_vertex() {
        let contents = vec![Content::text("hello"), Content::text("world")];
        let config = EmbedContentConfig {
            task_type: Some("retrieval".to_string()),
            title: Some("Title".to_string()),
            output_dimensionality: Some(8),
            ..Default::default()
        };
        let body = build_embed_body_gemini("gemini-1.5-pro", &contents, &config).unwrap();
        let requests = body.get("requests").and_then(Value::as_array).unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0].get("model").and_then(Value::as_str),
            Some("models/gemini-1.5-pro")
        );
        assert_eq!(
            requests[0].get("taskType").and_then(Value::as_str),
            Some("retrieval")
        );

        let bad_config = EmbedContentConfig {
            mime_type: Some("text/plain".to_string()),
            ..Default::default()
        };
        let err = build_embed_body_gemini("gemini-1.5-pro", &contents, &bad_config).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let vertex_config = EmbedContentConfig {
            task_type: Some("retrieval".to_string()),
            title: Some("Title".to_string()),
            mime_type: Some("text/plain".to_string()),
            auto_truncate: Some(true),
            output_dimensionality: Some(16),
        };
        let body = build_embed_body_vertex(&contents, &vertex_config).unwrap();
        let instances = body.get("instances").and_then(Value::as_array).unwrap();
        assert_eq!(instances.len(), 2);
        let params = body.get("parameters").and_then(Value::as_object).unwrap();
        assert_eq!(
            params.get("autoTruncate").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn test_build_generate_images_body_vertex_and_errors() {
        let config = GenerateImagesConfig {
            output_gcs_uri: Some("gs://out".to_string()),
            ..Default::default()
        };
        let err = build_generate_images_body(Backend::GeminiApi, "prompt", &config).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let vertex_config = GenerateImagesConfig {
            output_gcs_uri: Some("gs://out".to_string()),
            negative_prompt: Some("neg".to_string()),
            number_of_images: Some(2),
            aspect_ratio: Some("16:9".to_string()),
            guidance_scale: Some(7.5),
            seed: Some(42),
            safety_filter_level: Some(SafetyFilterLevel::BlockLowAndAbove),
            person_generation: Some(PersonGeneration::AllowAdult),
            include_safety_attributes: Some(true),
            include_rai_reason: Some(true),
            language: Some(ImagePromptLanguage::En),
            output_mime_type: Some("image/png".to_string()),
            output_compression_quality: Some(80),
            add_watermark: Some(true),
            labels: Some([("env".to_string(), "test".to_string())].into()),
            image_size: Some("512x512".to_string()),
            enhance_prompt: Some(true),
            ..Default::default()
        };

        let body = build_generate_images_body(Backend::VertexAi, "prompt", &vertex_config).unwrap();
        assert!(body.get("labels").is_some());
        let params = body.get("parameters").and_then(Value::as_object).unwrap();
        assert_eq!(params.get("sampleCount").and_then(Value::as_i64), Some(2));
        assert_eq!(params.get("seed").and_then(Value::as_i64), Some(42));
    }

    #[test]
    fn test_build_edit_and_upscale_image_body() {
        let image = Image {
            image_bytes: Some(vec![1, 2, 3]),
            mime_type: Some("image/png".to_string()),
            ..Default::default()
        };
        let reference = ReferenceImage {
            reference_image: Some(image.clone()),
            reference_id: Some(7),
            reference_type: Some(ReferenceImageType::ReferenceTypeMask),
            ..Default::default()
        };
        let config = EditImageConfig {
            output_mime_type: Some("image/jpeg".to_string()),
            output_compression_quality: Some(70),
            edit_mode: Some(EditMode::EditModeInpaintInsertion),
            base_steps: Some(4),
            ..Default::default()
        };
        let body = build_edit_image_body("prompt", &[reference], &config).unwrap();
        let instances = body.get("instances").and_then(Value::as_array).unwrap();
        let instance = instances[0].as_object().unwrap();
        assert!(instance.get("referenceImages").is_some());
        let params = body.get("parameters").and_then(Value::as_object).unwrap();
        assert!(params.get("outputOptions").is_some());
        assert!(params.get("editConfig").is_some());

        let upscale_config = UpscaleImageConfig {
            output_gcs_uri: Some("gs://out".to_string()),
            enhance_input_image: Some(true),
            image_preservation_factor: Some(0.3),
            ..Default::default()
        };
        let upscale_body = build_upscale_image_body(&image, "x2", &upscale_config).unwrap();
        let params = upscale_body
            .get("parameters")
            .and_then(Value::as_object)
            .unwrap();
        let sample_count = params.get("sampleCount").and_then(Value::as_i64);
        assert_eq!(sample_count, Some(1));
        assert!(params.get("upscaleConfig").is_some());
    }

    #[test]
    fn test_build_generate_videos_body_and_errors() {
        let source = GenerateVideosSource {
            prompt: Some("video".to_string()),
            image: Some(Image {
                image_bytes: Some(vec![4, 5, 6]),
                mime_type: Some("image/png".to_string()),
                ..Default::default()
            }),
            video: Some(Video {
                uri: Some("gs://video.mp4".to_string()),
                ..Default::default()
            }),
        };
        let bad_config = GenerateVideosConfig {
            mask: Some(VideoGenerationMask {
                mask_mode: Some(VideoGenerationMaskMode::Insert),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = build_generate_videos_body(Backend::GeminiApi, &source, &bad_config).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let config = GenerateVideosConfig {
            number_of_videos: Some(2),
            output_gcs_uri: Some("gs://out".to_string()),
            fps: Some(24),
            duration_seconds: Some(5),
            seed: Some(9),
            aspect_ratio: Some("16:9".to_string()),
            resolution: Some("720p".to_string()),
            person_generation: Some("allow".to_string()),
            pubsub_topic: Some("projects/p/topics/t".to_string()),
            negative_prompt: Some("noise".to_string()),
            enhance_prompt: Some(true),
            generate_audio: Some(true),
            compression_quality: Some(VideoCompressionQuality::Optimized),
            last_frame: Some(Image {
                gcs_uri: Some("gs://last.png".to_string()),
                ..Default::default()
            }),
            reference_images: Some(vec![VideoGenerationReferenceImage {
                image: Some(Image {
                    gcs_uri: Some("gs://ref.png".to_string()),
                    ..Default::default()
                }),
                reference_type: Some(VideoGenerationReferenceType::Style),
            }]),
            mask: Some(VideoGenerationMask {
                image: Some(Image {
                    gcs_uri: Some("gs://mask.png".to_string()),
                    ..Default::default()
                }),
                mask_mode: Some(VideoGenerationMaskMode::Insert),
            }),
            ..Default::default()
        };

        let body = build_generate_videos_body(Backend::VertexAi, &source, &config).unwrap();
        let params = body.get("parameters").and_then(Value::as_object).unwrap();
        assert_eq!(params.get("sampleCount").and_then(Value::as_i64), Some(2));
        assert!(params.get("pubsubTopic").is_some());
    }

    #[test]
    fn test_build_generate_images_body_gemini_unsupported_options() {
        let config = GenerateImagesConfig {
            negative_prompt: Some("neg".to_string()),
            ..Default::default()
        };
        assert!(build_generate_images_body(Backend::GeminiApi, "prompt", &config).is_err());

        let config = GenerateImagesConfig {
            seed: Some(7),
            ..Default::default()
        };
        assert!(build_generate_images_body(Backend::GeminiApi, "prompt", &config).is_err());

        let config = GenerateImagesConfig {
            add_watermark: Some(true),
            ..Default::default()
        };
        assert!(build_generate_images_body(Backend::GeminiApi, "prompt", &config).is_err());

        let config = GenerateImagesConfig {
            labels: Some([("k".to_string(), "v".to_string())].into()),
            ..Default::default()
        };
        assert!(build_generate_images_body(Backend::GeminiApi, "prompt", &config).is_err());

        let config = GenerateImagesConfig {
            enhance_prompt: Some(true),
            ..Default::default()
        };
        assert!(build_generate_images_body(Backend::GeminiApi, "prompt", &config).is_err());
    }

    #[test]
    fn test_build_generate_videos_body_gemini_unsupported_options() {
        let source = GenerateVideosSource {
            prompt: Some("video".to_string()),
            ..Default::default()
        };

        let config = GenerateVideosConfig {
            output_gcs_uri: Some("gs://out".to_string()),
            ..Default::default()
        };
        assert!(build_generate_videos_body(Backend::GeminiApi, &source, &config).is_err());

        let config = GenerateVideosConfig {
            fps: Some(24),
            ..Default::default()
        };
        assert!(build_generate_videos_body(Backend::GeminiApi, &source, &config).is_err());

        let config = GenerateVideosConfig {
            seed: Some(9),
            ..Default::default()
        };
        assert!(build_generate_videos_body(Backend::GeminiApi, &source, &config).is_err());

        let config = GenerateVideosConfig {
            pubsub_topic: Some("projects/p/topics/t".to_string()),
            ..Default::default()
        };
        assert!(build_generate_videos_body(Backend::GeminiApi, &source, &config).is_err());

        let config = GenerateVideosConfig {
            generate_audio: Some(true),
            ..Default::default()
        };
        assert!(build_generate_videos_body(Backend::GeminiApi, &source, &config).is_err());

        let config = GenerateVideosConfig {
            compression_quality: Some(VideoCompressionQuality::Optimized),
            ..Default::default()
        };
        assert!(build_generate_videos_body(Backend::GeminiApi, &source, &config).is_err());
    }

    #[test]
    fn test_build_embed_body_gemini_auto_truncate_error() {
        let contents = vec![Content::text("hi")];
        let config = EmbedContentConfig {
            auto_truncate: Some(true),
            ..Default::default()
        };
        let err = build_embed_body_gemini("gemini-1.5-pro", &contents, &config).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_edit_image_body_full_options() {
        let edit_config = EditImageConfig {
            output_gcs_uri: Some("gs://out".to_string()),
            negative_prompt: Some("neg".to_string()),
            number_of_images: Some(3),
            aspect_ratio: Some("1:1".to_string()),
            guidance_scale: Some(1.5),
            seed: Some(7),
            safety_filter_level: Some(SafetyFilterLevel::BlockLowAndAbove),
            person_generation: Some(PersonGeneration::AllowAdult),
            include_safety_attributes: Some(true),
            include_rai_reason: Some(true),
            language: Some(ImagePromptLanguage::En),
            output_mime_type: Some("image/png".to_string()),
            output_compression_quality: Some(90),
            add_watermark: Some(true),
            labels: Some([("k".to_string(), "v".to_string())].into()),
            edit_mode: Some(EditMode::EditModeInpaintInsertion),
            base_steps: Some(2),
            ..Default::default()
        };
        let body = build_edit_image_body("prompt", &[], &edit_config).unwrap();
        let params = body.get("parameters").and_then(Value::as_object).unwrap();
        assert!(params.get("storageUri").is_some());
        assert!(params.get("outputOptions").is_some());
        assert!(params.get("editConfig").is_some());
    }

    #[test]
    fn test_build_recontext_image_body_full_options() {
        let recontext_config = RecontextImageConfig {
            number_of_images: Some(2),
            base_steps: Some(3),
            output_gcs_uri: Some("gs://out".to_string()),
            seed: Some(5),
            safety_filter_level: Some(SafetyFilterLevel::BlockLowAndAbove),
            person_generation: Some(PersonGeneration::AllowAdult),
            add_watermark: Some(true),
            output_mime_type: Some("image/png".to_string()),
            output_compression_quality: Some(70),
            enhance_prompt: Some(true),
            labels: Some([("k".to_string(), "v".to_string())].into()),
            ..Default::default()
        };
        let source = RecontextImageSource {
            prompt: Some("test".to_string()),
            person_image: Some(Image {
                gcs_uri: Some("gs://person.png".to_string()),
                ..Default::default()
            }),
            product_images: Some(vec![ProductImage {
                product_image: Some(Image {
                    gcs_uri: Some("gs://product.png".to_string()),
                    ..Default::default()
                }),
            }]),
        };
        let body = build_recontext_image_body(&source, &recontext_config).unwrap();
        assert!(body.get("labels").is_some());
        assert!(body.get("parameters").is_some());
    }

    #[test]
    fn test_build_segment_image_body_full_options() {
        let segment_source = SegmentImageSource {
            prompt: Some("foreground".to_string()),
            image: Some(Image {
                gcs_uri: Some("gs://input.png".to_string()),
                ..Default::default()
            }),
            scribble_image: Some(ScribbleImage {
                image: Some(Image {
                    gcs_uri: Some("gs://scribble.png".to_string()),
                    ..Default::default()
                }),
            }),
        };
        let segment_config = SegmentImageConfig {
            mode: Some(SegmentMode::Foreground),
            max_predictions: Some(2),
            confidence_threshold: Some(0.5),
            mask_dilation: Some(0.1),
            binary_color_threshold: Some(0.3),
            labels: Some([("k".to_string(), "v".to_string())].into()),
            ..Default::default()
        };
        let body = build_segment_image_body(&segment_source, &segment_config).unwrap();
        assert!(body.get("labels").is_some());
        assert!(body.get("parameters").is_some());
    }

    #[test]
    fn test_build_upscale_image_body_full_options() {
        let upscale_config = UpscaleImageConfig {
            mode: Some("upscale".to_string()),
            number_of_images: Some(2),
            output_gcs_uri: Some("gs://out".to_string()),
            safety_filter_level: Some(SafetyFilterLevel::BlockLowAndAbove),
            person_generation: Some(PersonGeneration::AllowAdult),
            include_rai_reason: Some(true),
            output_mime_type: Some("image/png".to_string()),
            output_compression_quality: Some(80),
            enhance_input_image: Some(true),
            image_preservation_factor: Some(0.2),
            labels: Some([("k".to_string(), "v".to_string())].into()),
            ..Default::default()
        };
        let image = Image {
            image_bytes: Some(vec![1, 2, 3]),
            mime_type: Some("image/png".to_string()),
            ..Default::default()
        };
        let body = build_upscale_image_body(&image, "x2", &upscale_config).unwrap();
        let params = body.get("parameters").and_then(Value::as_object).unwrap();
        assert_eq!(params.get("sampleCount").and_then(Value::as_i64), Some(2));
        assert!(body.get("labels").is_some());
    }
}
