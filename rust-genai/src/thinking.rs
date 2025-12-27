//! Thinking support and thought signature validation.

use rust_genai_types::content::{Content, PartKind, Role};
use rust_genai_types::models::GenerateContentConfig;

use crate::error::{Error, Result};

/// Thought Signature 验证器。
pub struct ThoughtSignatureValidator {
    model: String,
}

impl ThoughtSignatureValidator {
    /// 创建验证器。
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }

    /// 验证对话历史中的 thought signatures。
    pub fn validate(&self, contents: &[Content]) -> Result<()> {
        if !is_gemini_3(&self.model) {
            return Ok(());
        }

        let current_turn_start = find_current_turn_start(contents);

        for content in &contents[current_turn_start..] {
            if content.role != Some(Role::Model) {
                continue;
            }

            let function_parts: Vec<_> = content
                .parts
                .iter()
                .filter(|part| matches!(part.kind, PartKind::FunctionCall { .. }))
                .collect();

            if function_parts.is_empty() {
                continue;
            }

            if function_parts[0].thought_signature.is_none() {
                return Err(Error::MissingThoughtSignature {
                    message: "First function call missing thought_signature".into(),
                });
            }

            for part in function_parts.iter().skip(1) {
                if part.thought_signature.is_some() {
                    return Err(Error::MissingThoughtSignature {
                        message: "Only the first function call may include thought_signature"
                            .into(),
                    });
                }
            }
        }

        Ok(())
    }
}

/// Gemini 3 温度检查。
pub fn validate_temperature(model: &str, config: &GenerateContentConfig) -> Result<()> {
    if !is_gemini_3(model) {
        return Ok(());
    }

    if let Some(temperature) = config
        .generation_config
        .as_ref()
        .and_then(|cfg| cfg.temperature)
    {
        if temperature < 1.0 {
            eprintln!(
                "Warning: Gemini 3 temperature {} < 1.0 may cause looping; use 1.0",
                temperature
            );
        }
    }

    Ok(())
}

fn is_gemini_3(model: &str) -> bool {
    model
        .rsplit('/')
        .next()
        .map(|name| name.starts_with("gemini-3"))
        .unwrap_or(false)
}

fn find_current_turn_start(contents: &[Content]) -> usize {
    for (idx, content) in contents.iter().enumerate().rev() {
        if content.role != Some(Role::User) {
            continue;
        }
        let has_text = content
            .parts
            .iter()
            .any(|part| matches!(part.kind, PartKind::Text { .. }));
        if has_text {
            return idx;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_genai_types::content::{FunctionCall, Part};

    #[test]
    fn test_thought_signature_validation_gemini3() {
        let validator = ThoughtSignatureValidator::new("gemini-3-pro-preview");
        let contents = vec![
            Content::user("Check flight AA100"),
            Content::from_parts(
                vec![Part::function_call(FunctionCall {
                    id: None,
                    name: Some("check_flight".into()),
                    args: None,
                    partial_args: None,
                    will_continue: None,
                })],
                Role::Model,
            ),
        ];

        assert!(validator.validate(&contents).is_err());
    }

    #[test]
    fn test_temperature_warning_gemini3() {
        let config = GenerateContentConfig {
            generation_config: Some(rust_genai_types::config::GenerationConfig {
                temperature: Some(0.5),
                ..Default::default()
            }),
            ..Default::default()
        };
        validate_temperature("gemini-3-flash-preview", &config).unwrap();
    }
}
