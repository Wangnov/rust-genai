//! Model capability checks and feature gating.

use rust_genai_types::content::{Content, PartKind};
use rust_genai_types::tool::Tool;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct ModelCapabilities {
    pub supports_function_response_media: bool,
    pub supports_code_execution_images: bool,
    pub supports_native_audio: bool,
    pub supports_thinking: bool,
}

pub fn capabilities_for(model: &str) -> ModelCapabilities {
    let name = normalize_model_name(model);
    let is_gemini_3 = name.starts_with("gemini-3");
    let supports_native_audio = name.contains("native-audio");
    ModelCapabilities {
        supports_function_response_media: is_gemini_3,
        supports_code_execution_images: is_gemini_3,
        supports_native_audio,
        supports_thinking: is_gemini_3 || name.contains("thinking"),
    }
}

pub fn validate_function_response_media(model: &str, contents: &[Content]) -> Result<()> {
    if !has_function_response_media(contents) {
        return Ok(());
    }
    let caps = capabilities_for(model);
    if !caps.supports_function_response_media {
        return Err(Error::InvalidConfig {
            message: format!("Model {model} does not support media in FunctionResponse parts"),
        });
    }
    Ok(())
}

pub fn validate_code_execution_image_inputs(
    model: &str,
    contents: &[Content],
    tools: Option<&[Tool]>,
) -> Result<()> {
    if !has_code_execution_tool(tools) || !has_image_inputs(contents) {
        return Ok(());
    }
    let caps = capabilities_for(model);
    if !caps.supports_code_execution_images {
        return Err(Error::InvalidConfig {
            message: format!("Model {model} does not support code execution with image inputs"),
        });
    }
    Ok(())
}

fn normalize_model_name(model: &str) -> String {
    model.rsplit('/').next().unwrap_or(model).to_string()
}

fn has_function_response_media(contents: &[Content]) -> bool {
    for content in contents {
        for part in &content.parts {
            if let PartKind::FunctionResponse { function_response } = &part.kind {
                if let Some(parts) = &function_response.parts {
                    if parts
                        .iter()
                        .any(|p| p.inline_data.is_some() || p.file_data.is_some())
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn has_code_execution_tool(tools: Option<&[Tool]>) -> bool {
    tools
        .map(|items| items.iter().any(|tool| tool.code_execution.is_some()))
        .unwrap_or(false)
}

fn has_image_inputs(contents: &[Content]) -> bool {
    for content in contents {
        for part in &content.parts {
            match &part.kind {
                PartKind::InlineData { inline_data } => {
                    if inline_data.mime_type.starts_with("image/") {
                        return true;
                    }
                }
                PartKind::FileData { file_data } => {
                    if file_data.mime_type.starts_with("image/") {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }
    false
}
