//! Model capability checks and feature gating.

use rust_genai_types::content::{Content, PartKind};
use rust_genai_types::tool::Tool;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, Default)]
pub struct ModelCapabilities {
    flags: u8,
}

impl ModelCapabilities {
    const FUNCTION_RESPONSE_MEDIA: u8 = 1 << 0;
    const CODE_EXECUTION_IMAGES: u8 = 1 << 1;
    const NATIVE_AUDIO: u8 = 1 << 2;
    const THINKING: u8 = 1 << 3;

    const fn new(flags: u8) -> Self {
        Self { flags }
    }

    #[must_use]
    pub const fn supports_function_response_media(self) -> bool {
        self.flags & Self::FUNCTION_RESPONSE_MEDIA != 0
    }

    #[must_use]
    pub const fn supports_code_execution_images(self) -> bool {
        self.flags & Self::CODE_EXECUTION_IMAGES != 0
    }

    #[must_use]
    pub const fn supports_native_audio(self) -> bool {
        self.flags & Self::NATIVE_AUDIO != 0
    }

    #[must_use]
    pub const fn supports_thinking(self) -> bool {
        self.flags & Self::THINKING != 0
    }
}

#[must_use]
pub fn capabilities_for(model: &str) -> ModelCapabilities {
    let name = normalize_model_name(model);
    let is_gemini_3 = name.starts_with("gemini-3");
    let supports_native_audio = name.contains("native-audio");
    let mut flags = 0;
    if is_gemini_3 {
        flags |= ModelCapabilities::FUNCTION_RESPONSE_MEDIA;
        flags |= ModelCapabilities::CODE_EXECUTION_IMAGES;
    }
    if supports_native_audio {
        flags |= ModelCapabilities::NATIVE_AUDIO;
    }
    if is_gemini_3 || name.contains("thinking") {
        flags |= ModelCapabilities::THINKING;
    }
    ModelCapabilities::new(flags)
}

/// # Errors
/// 当模型不支持功能响应多媒体时返回错误。
pub fn validate_function_response_media(model: &str, contents: &[Content]) -> Result<()> {
    if !has_function_response_media(contents) {
        return Ok(());
    }
    let caps = capabilities_for(model);
    if !caps.supports_function_response_media() {
        return Err(Error::InvalidConfig {
            message: format!("Model {model} does not support media in FunctionResponse parts"),
        });
    }
    Ok(())
}

/// # Errors
/// 当模型不支持带图像的代码执行时返回错误。
pub fn validate_code_execution_image_inputs(
    model: &str,
    contents: &[Content],
    tools: Option<&[Tool]>,
) -> Result<()> {
    if !has_code_execution_tool(tools) || !has_image_inputs(contents) {
        return Ok(());
    }
    let caps = capabilities_for(model);
    if !caps.supports_code_execution_images() {
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
    tools.is_some_and(|items| items.iter().any(|tool| tool.code_execution.is_some()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_genai_types::content::{
        Content, FunctionResponse, FunctionResponseBlob, FunctionResponseFileData,
        FunctionResponsePart, Part, Role,
    };
    use rust_genai_types::tool::Tool;
    use serde_json::json;

    #[test]
    fn capabilities_detect_features() {
        let caps = capabilities_for("models/gemini-3-thinking");
        assert!(caps.supports_function_response_media());
        assert!(caps.supports_code_execution_images());
        assert!(caps.supports_thinking());

        let caps = capabilities_for("gemini-2.0-flash-native-audio");
        assert!(caps.supports_native_audio());
        assert!(!caps.supports_function_response_media());
    }

    #[test]
    fn validate_function_response_media_blocks_unsupported_models() {
        let response = FunctionResponse {
            will_continue: None,
            scheduling: None,
            parts: Some(vec![rust_genai_types::content::FunctionResponsePart {
                inline_data: Some(rust_genai_types::content::FunctionResponseBlob {
                    mime_type: "image/png".into(),
                    data: vec![1, 2, 3],
                    display_name: None,
                }),
                file_data: None,
            }]),
            id: Some("id".into()),
            name: Some("fn".into()),
            response: Some(json!({"ok": true})),
        };
        let content = Content::from_parts(vec![Part::function_response(response)], Role::Model);
        let result = validate_function_response_media("gemini-2.0-flash", &[content]);
        assert!(result.is_err());

        let ok = validate_function_response_media("gemini-3", &[]);
        assert!(ok.is_ok());
    }

    #[test]
    fn validate_code_execution_image_inputs_blocks_unsupported_models() {
        let tool = Tool {
            code_execution: Some(rust_genai_types::tool::CodeExecution {}),
            ..Default::default()
        };
        let content = Content::from_parts(
            vec![Part::inline_data(vec![1, 2, 3], "image/png")],
            Role::User,
        );
        let result = validate_code_execution_image_inputs(
            "gemini-2.0-flash",
            std::slice::from_ref(&content),
            Some(&[tool]),
        );
        assert!(result.is_err());

        let ok = validate_code_execution_image_inputs(
            "gemini-3",
            std::slice::from_ref(&content),
            Some(&[Tool {
                code_execution: Some(rust_genai_types::tool::CodeExecution {}),
                ..Default::default()
            }]),
        );
        assert!(ok.is_ok());
    }

    #[test]
    fn has_function_response_media_detects_parts() {
        let response = FunctionResponse {
            will_continue: None,
            scheduling: None,
            parts: Some(vec![
                FunctionResponsePart {
                    inline_data: Some(FunctionResponseBlob {
                        mime_type: "image/png".into(),
                        data: vec![1],
                        display_name: None,
                    }),
                    file_data: None,
                },
                FunctionResponsePart {
                    inline_data: None,
                    file_data: Some(FunctionResponseFileData {
                        file_uri: "files/abc".into(),
                        mime_type: "image/png".into(),
                        display_name: None,
                    }),
                },
            ]),
            id: Some("id".into()),
            name: Some("fn".into()),
            response: Some(json!({"ok": true})),
        };
        let content = Content::from_parts(vec![Part::function_response(response)], Role::Model);
        assert!(has_function_response_media(&[content]));
    }

    #[test]
    fn has_image_inputs_detects_inline_and_file() {
        let inline = Content::from_parts(
            vec![Part::inline_data(vec![1, 2, 3], "image/png")],
            Role::User,
        );
        let file = Content::from_parts(vec![Part::file_data("files/abc", "image/png")], Role::User);
        assert!(has_image_inputs(&[inline]));
        assert!(has_image_inputs(&[file]));
    }
}
