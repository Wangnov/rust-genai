use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::base64_serde;
use crate::enums::{FunctionResponseScheduling, Language, Outcome, PartMediaResolutionLevel};

#[cfg(feature = "mcp")]
use rmcp::model::CallToolResult;

/// 对话内容。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    /// 角色：user/model/function。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// 消息内容片段。
    #[serde(default)]
    pub parts: Vec<Part>,
}

impl Content {
    /// 创建用户文本消息。
    pub fn user(text: impl Into<String>) -> Self {
        Self::from_text(text, Role::User)
    }

    /// 创建模型文本消息。
    pub fn model(text: impl Into<String>) -> Self {
        Self::from_text(text, Role::Model)
    }

    /// 创建文本消息。
    pub fn text(text: impl Into<String>) -> Self {
        Self::from_text(text, Role::User)
    }

    /// 从 parts 构建内容。
    pub fn from_parts(parts: Vec<Part>, role: Role) -> Self {
        Self {
            role: Some(role),
            parts,
        }
    }

    /// 提取第一段文本。
    pub fn first_text(&self) -> Option<&str> {
        self.parts.iter().find_map(|part| part.text_value())
    }

    fn from_text(text: impl Into<String>, role: Role) -> Self {
        Self {
            role: Some(role),
            parts: vec![Part::text(text)],
        }
    }
}

/// 内容角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Model,
    Function,
}

/// 内容部分。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    /// 具体内容变体。
    #[serde(flatten)]
    pub kind: PartKind,
    /// 是否为思考内容。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought: Option<bool>,
    /// 思考签名（base64 编码）。
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "base64_serde::option"
    )]
    pub thought_signature: Option<Vec<u8>>,
    /// 媒体分辨率设置（按 part）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_resolution: Option<PartMediaResolution>,
    /// 视频元数据。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_metadata: Option<VideoMetadata>,
}

impl Part {
    /// 创建文本 Part。
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            kind: PartKind::Text { text: text.into() },
            thought: None,
            thought_signature: None,
            media_resolution: None,
            video_metadata: None,
        }
    }

    /// 创建内联二进制数据 Part。
    pub fn inline_data(data: Vec<u8>, mime_type: impl Into<String>) -> Self {
        Self {
            kind: PartKind::InlineData {
                inline_data: Blob {
                    mime_type: mime_type.into(),
                    data,
                    display_name: None,
                },
            },
            thought: None,
            thought_signature: None,
            media_resolution: None,
            video_metadata: None,
        }
    }

    /// 创建文件 URI Part。
    pub fn file_data(file_uri: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            kind: PartKind::FileData {
                file_data: FileData {
                    file_uri: file_uri.into(),
                    mime_type: mime_type.into(),
                    display_name: None,
                },
            },
            thought: None,
            thought_signature: None,
            media_resolution: None,
            video_metadata: None,
        }
    }

    /// 创建函数调用 Part。
    pub fn function_call(function_call: FunctionCall) -> Self {
        Self {
            kind: PartKind::FunctionCall { function_call },
            thought: None,
            thought_signature: None,
            media_resolution: None,
            video_metadata: None,
        }
    }

    /// 创建函数响应 Part。
    pub fn function_response(function_response: FunctionResponse) -> Self {
        Self {
            kind: PartKind::FunctionResponse { function_response },
            thought: None,
            thought_signature: None,
            media_resolution: None,
            video_metadata: None,
        }
    }

    /// 创建可执行代码 Part。
    pub fn executable_code(code: impl Into<String>, language: Language) -> Self {
        Self {
            kind: PartKind::ExecutableCode {
                executable_code: ExecutableCode {
                    code: code.into(),
                    language,
                },
            },
            thought: None,
            thought_signature: None,
            media_resolution: None,
            video_metadata: None,
        }
    }

    /// 创建代码执行结果 Part。
    pub fn code_execution_result(outcome: Outcome, output: impl Into<String>) -> Self {
        Self {
            kind: PartKind::CodeExecutionResult {
                code_execution_result: CodeExecutionResult {
                    outcome,
                    output: Some(output.into()),
                },
            },
            thought: None,
            thought_signature: None,
            media_resolution: None,
            video_metadata: None,
        }
    }

    /// 设置是否为思考内容。
    pub fn with_thought(mut self, thought: bool) -> Self {
        self.thought = Some(thought);
        self
    }

    /// 设置 thought signature。
    pub fn with_thought_signature(mut self, signature: Vec<u8>) -> Self {
        self.thought_signature = Some(signature);
        self
    }

    /// 设置媒体分辨率。
    pub fn with_media_resolution(mut self, resolution: PartMediaResolution) -> Self {
        self.media_resolution = Some(resolution);
        self
    }

    /// 设置视频元数据。
    pub fn with_video_metadata(mut self, metadata: VideoMetadata) -> Self {
        self.video_metadata = Some(metadata);
        self
    }

    /// 获取文本内容（仅当为 Text Part）。
    pub fn text_value(&self) -> Option<&str> {
        match &self.kind {
            PartKind::Text { text } => Some(text.as_str()),
            _ => None,
        }
    }

    /// 获取函数调用引用（仅当为 FunctionCall Part）。
    pub fn function_call_ref(&self) -> Option<&FunctionCall> {
        match &self.kind {
            PartKind::FunctionCall { function_call } => Some(function_call),
            _ => None,
        }
    }
}

/// 内容部分的具体变体。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum PartKind {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: Blob,
    },
    FileData {
        #[serde(rename = "fileData")]
        file_data: FileData,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },
    ExecutableCode {
        #[serde(rename = "executableCode")]
        executable_code: ExecutableCode,
    },
    CodeExecutionResult {
        #[serde(rename = "codeExecutionResult")]
        code_execution_result: CodeExecutionResult,
    },
}

/// 媒体分辨率设置（按 part，Gemini 3 支持）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartMediaResolution {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<PartMediaResolutionLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_tokens: Option<i32>,
}

/// 二进制数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    pub mime_type: String,
    #[serde(with = "base64_serde")]
    pub data: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// URI 文件数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileData {
    pub file_uri: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// 部分参数值（函数调用流式参数）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialArg {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub null_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bool_value: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub will_continue: Option<bool>,
}

/// 函数调用。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_args: Option<Vec<PartialArg>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub will_continue: Option<bool>,
}

/// 函数响应内容中的二进制数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponseBlob {
    pub mime_type: String,
    #[serde(with = "base64_serde")]
    pub data: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// 函数响应内容中的文件引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponseFileData {
    pub file_uri: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// 函数响应的多模态 part。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponsePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<FunctionResponseBlob>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<FunctionResponseFileData>,
}

/// 函数响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub will_continue: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduling: Option<FunctionResponseScheduling>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<FunctionResponsePart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
}

impl FunctionResponse {
    /// 从 MCP CallToolResult 构造 FunctionResponse（需要启用 `mcp` feature）。
    #[cfg(feature = "mcp")]
    pub fn from_mcp_response(
        name: impl Into<String>,
        response: &CallToolResult,
    ) -> Result<Self, serde_json::Error> {
        let value = serde_json::to_value(response)?;
        let is_error = response.is_error.unwrap_or(false);
        let response_value = if is_error {
            let mut wrapper = serde_json::Map::new();
            wrapper.insert("error".to_string(), value);
            Value::Object(wrapper)
        } else {
            value
        };
        Ok(Self {
            will_continue: None,
            scheduling: None,
            parts: None,
            id: None,
            name: Some(name.into()),
            response: Some(response_value),
        })
    }
}

/// 可执行代码。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutableCode {
    pub code: String,
    pub language: Language,
}

/// 代码执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeExecutionResult {
    pub outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// 视频元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_offset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_offset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn content_roundtrip() {
        let content = Content::user("hello");
        let json = serde_json::to_string(&content).unwrap();
        let decoded: Content = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.parts.len(), 1);
    }

    #[test]
    fn blob_base64_serialization() {
        let blob = Blob {
            mime_type: "image/png".into(),
            data: vec![1, 2, 3],
            display_name: None,
        };
        let value = serde_json::to_value(&blob).unwrap();
        assert!(value["data"].is_string());
    }

    #[test]
    fn function_response_media_roundtrip() {
        let response = FunctionResponse {
            will_continue: None,
            scheduling: None,
            parts: Some(vec![FunctionResponsePart {
                inline_data: Some(FunctionResponseBlob {
                    mime_type: "image/png".into(),
                    data: vec![1, 2, 3],
                    display_name: None,
                }),
                file_data: None,
            }]),
            id: Some("fn-1".into()),
            name: Some("render_chart".into()),
            response: None,
        };

        let part = Part::function_response(response);
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("inlineData"));
    }

    #[test]
    fn function_call_part_deserializes_from_camel_case() {
        let value = json!({
            "functionCall": {
                "name": "add_numbers",
                "args": { "a": 2.5, "b": 3.1 }
            },
            "thoughtSignature": "AQID"
        });
        let part: Part = serde_json::from_value(value).unwrap();
        let call = part.function_call_ref().expect("missing function call");
        assert_eq!(call.name.as_deref(), Some("add_numbers"));
    }
}
