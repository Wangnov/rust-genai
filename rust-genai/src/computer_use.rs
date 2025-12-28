//! Computer Use helpers.

use serde::{Deserialize, Serialize};

use rust_genai_types::tool::FunctionDeclaration;

/// 预置的 Computer Use UI 动作名称。
pub const COMPUTER_USE_ACTIONS: &[&str] = &[
    "open_web_browser",
    "wait_5_seconds",
    "go_back",
    "go_forward",
    "search",
    "navigate",
    "click_at",
    "hover_at",
    "type_text_at",
    "key_combination",
    "scroll_document",
    "scroll_at",
    "drag_and_drop",
];

/// 生成预置动作的 `FunctionDeclaration` 列表。
#[must_use]
pub fn computer_use_function_declarations() -> Vec<FunctionDeclaration> {
    COMPUTER_USE_ACTIONS
        .iter()
        .map(|name| FunctionDeclaration {
            name: (*name).to_string(),
            description: None,
            parameters: None,
            parameters_json_schema: None,
            response: None,
            response_json_schema: None,
            behavior: None,
        })
        .collect()
}

/// 坐标归一化（像素 -> 0-999）。
#[must_use]
pub fn normalize_coordinate(pixel: i32, screen_dimension: i32) -> i32 {
    if screen_dimension <= 0 {
        return 0;
    }
    let numerator = i64::from(pixel) * 1000;
    let denominator = i64::from(screen_dimension);
    let value = numerator / denominator;
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

/// 坐标反归一化（0-999 -> 像素）。
#[must_use]
pub fn denormalize_coordinate(normalized: i32, screen_dimension: i32) -> i32 {
    if screen_dimension <= 0 {
        return 0;
    }
    let numerator = i64::from(normalized) * i64::from(screen_dimension);
    let value = numerator / 1000;
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

/// Safety decision 信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SafetyDecision {
    pub decision: SafetyDecisionType,
    pub explanation: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyDecisionType {
    RequireConfirmation,
    Regular,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_computer_use_coordinate_normalization() {
        assert_eq!(normalize_coordinate(720, 1440), 500);
        assert_eq!(denormalize_coordinate(500, 1440), 720);
    }

    #[test]
    fn test_computer_use_actions_and_zero_dimension() {
        let declarations = computer_use_function_declarations();
        assert_eq!(declarations.len(), COMPUTER_USE_ACTIONS.len());
        assert_eq!(declarations[0].name, COMPUTER_USE_ACTIONS[0]);
        assert_eq!(normalize_coordinate(10, 0), 0);
        assert_eq!(denormalize_coordinate(10, -1), 0);
    }
}
