//! MCP (Model Context Protocol) integration helpers.
//!
//! This module is experimental and behind the `mcp` feature.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures_util::future::BoxFuture;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::{CallToolRequestParam, Tool as McpTool};
use rmcp::service::{Peer, RoleClient};
use rust_genai_types::content::{FunctionCall, FunctionResponse, Part};
use rust_genai_types::enums::Behavior;
use rust_genai_types::tool::{FunctionDeclaration, Tool};
use serde_json::{Map, Value};

use crate::afc::CallableTool;
use crate::error::{Error, Result};

/// MCP 观测标记（附加在 x-goog-api-client）。
pub const MCP_LABEL: &str = "mcp_used/unknown";
/// Google API Client header 名称。
pub const MCP_USAGE_HEADER: &str = "x-goog-api-client";

static MCP_TOOL_USAGE: AtomicBool = AtomicBool::new(false);

/// MCP 可调用工具配置。
#[derive(Debug, Clone, Default)]
pub struct McpCallableToolConfig {
    /// 模型在调用工具后的行为。
    pub behavior: Option<Behavior>,
    /// 远程调用超时时间。
    pub timeout: Option<Duration>,
}

/// 追加 MCP 使用标记到 headers（可用于观测/遥测）。
pub fn set_mcp_usage_header(headers: &mut HashMap<String, String>) {
    mark_mcp_tool_usage();
    let entry = headers.entry(MCP_USAGE_HEADER.to_string()).or_default();
    if entry.contains(MCP_LABEL) {
        return;
    }
    if entry.is_empty() {
        *entry = MCP_LABEL.to_string();
    } else {
        *entry = format!("{} {}", entry.trim(), MCP_LABEL);
    }
}

/// 自动在 HeaderMap 中追加 MCP 使用标记（仅当检测到 MCP 工具使用）。
pub fn append_mcp_usage_header(headers: &mut HeaderMap) -> Result<()> {
    if !has_mcp_tool_usage() {
        return Ok(());
    }
    let header_name = HeaderName::from_static(MCP_USAGE_HEADER);
    let existing_value = headers
        .get(&header_name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if existing_value.contains(MCP_LABEL) {
        return Ok(());
    }
    let next_value = if existing_value.is_empty() {
        MCP_LABEL.to_string()
    } else {
        format!("{} {}", existing_value.trim(), MCP_LABEL)
    };
    let header_value = HeaderValue::from_str(&next_value).map_err(|_| Error::InvalidConfig {
        message: "Invalid x-goog-api-client header value".into(),
    })?;
    headers.insert(header_name, header_value);
    Ok(())
}

/// MCP 工具使用标记（用于请求链路自动注入 header）。
pub fn has_mcp_tool_usage() -> bool {
    MCP_TOOL_USAGE.load(Ordering::Relaxed)
}

fn mark_mcp_tool_usage() {
    MCP_TOOL_USAGE.store(true, Ordering::Relaxed);
}

/// 将 MCP tools 转换为单个 Gemini Tool（包含 FunctionDeclaration 列表）。
pub fn mcp_to_tool(mcp_tools: &[McpTool], config: &McpCallableToolConfig) -> Result<Tool> {
    mark_mcp_tool_usage();
    let mut declarations = Vec::with_capacity(mcp_tools.len());
    let mut seen = HashSet::new();
    for tool in mcp_tools {
        let name = tool.name.to_string();
        if !seen.insert(name.clone()) {
            return Err(Error::InvalidConfig {
                message: format!(
                    "Duplicate function name {name} found in MCP tools. Please ensure function names are unique."
                ),
            });
        }
        declarations.push(mcp_tool_to_declaration(tool, config));
    }
    Ok(Tool {
        function_declarations: Some(declarations),
        ..Tool::default()
    })
}

fn mcp_tool_to_declaration(tool: &McpTool, config: &McpCallableToolConfig) -> FunctionDeclaration {
    FunctionDeclaration {
        name: tool.name.to_string(),
        description: tool.description.as_ref().map(|value| value.to_string()),
        parameters: None,
        parameters_json_schema: Some(Value::Object(tool.input_schema.as_ref().clone())),
        response: None,
        response_json_schema: tool
            .output_schema
            .as_ref()
            .map(|schema| Value::Object(schema.as_ref().clone())),
        behavior: config.behavior,
    }
}

/// MCP 可调用工具（桥接 MCP 与 Gemini Function Calling）。
#[derive(Debug, Clone)]
pub struct McpCallableTool {
    clients: Vec<Peer<RoleClient>>,
    config: McpCallableToolConfig,
    mcp_tools: Vec<McpTool>,
    function_name_to_client: HashMap<String, Peer<RoleClient>>,
    initialized: bool,
}

impl McpCallableTool {
    /// 创建新的 McpCallableTool。
    pub fn new(clients: Vec<Peer<RoleClient>>, config: McpCallableToolConfig) -> Self {
        mark_mcp_tool_usage();
        Self {
            clients,
            config,
            mcp_tools: Vec::new(),
            function_name_to_client: HashMap::new(),
            initialized: false,
        }
    }

    /// 初始化 MCP 工具列表并构建函数名映射。
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        let mut tools = Vec::new();
        let mut function_map = HashMap::new();
        for client in &self.clients {
            let client_tools = client.list_all_tools().await?;
            for tool in client_tools {
                let name = tool.name.to_string();
                if function_map.contains_key(&name) {
                    return Err(Error::InvalidConfig {
                        message: format!(
                            "Duplicate function name {name} found in MCP tools. Please ensure function names are unique."
                        ),
                    });
                }
                function_map.insert(name, client.clone());
                tools.push(tool);
            }
        }
        self.mcp_tools = tools;
        self.function_name_to_client = function_map;
        self.initialized = true;
        Ok(())
    }

    /// 获取 Gemini Tool 定义（会自动初始化）。
    pub async fn tool(&mut self) -> Result<Tool> {
        self.initialize().await?;
        mcp_to_tool(&self.mcp_tools, &self.config)
    }

    /// 执行 MCP 工具调用并返回 FunctionResponse Parts。
    pub async fn call_tool(&mut self, function_calls: &[FunctionCall]) -> Result<Vec<Part>> {
        self.initialize().await?;
        let mut parts = Vec::new();
        for call in function_calls {
            let name = match call.name.as_ref() {
                Some(name) => name,
                None => continue,
            };
            let client = match self.function_name_to_client.get(name) {
                Some(client) => client,
                None => continue,
            };

            let arguments = normalize_mcp_arguments(call.args.as_ref(), name)?;
            let request = CallToolRequestParam {
                name: name.clone().into(),
                arguments,
            };

            let result = match self.config.timeout {
                Some(timeout) => {
                    match tokio::time::timeout(timeout, client.call_tool(request)).await {
                        Ok(result) => result?,
                        Err(_) => {
                            return Err(Error::Timeout {
                                message: format!("Timed out calling MCP tool: {name}"),
                            })
                        }
                    }
                }
                None => client.call_tool(request).await?,
            };

            let response_value = mcp_result_to_value(&result)?;
            let function_response = FunctionResponse {
                will_continue: None,
                scheduling: None,
                parts: None,
                id: call.id.clone(),
                name: Some(name.clone()),
                response: Some(response_value),
            };
            parts.push(Part::function_response(function_response));
        }
        Ok(parts)
    }
}

impl CallableTool for McpCallableTool {
    fn tool(&mut self) -> BoxFuture<'_, Result<Tool>> {
        Box::pin(async move { McpCallableTool::tool(self).await })
    }

    fn call_tool(&mut self, function_calls: Vec<FunctionCall>) -> BoxFuture<'_, Result<Vec<Part>>> {
        Box::pin(async move { McpCallableTool::call_tool(self, &function_calls).await })
    }
}

fn normalize_mcp_arguments(
    args: Option<&Value>,
    function_name: &str,
) -> Result<Option<Map<String, Value>>> {
    match args {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(Value::Object(map)) => Ok(Some(map.clone())),
        Some(_) => Err(Error::InvalidConfig {
            message: format!("MCP tool call expects object arguments for {function_name}"),
        }),
    }
}

fn mcp_result_to_value(result: &rmcp::model::CallToolResult) -> Result<Value> {
    let value = serde_json::to_value(result)?;
    let is_error = result.is_error.unwrap_or(false);
    if is_error {
        let mut wrapper = Map::new();
        wrapper.insert("error".to_string(), value);
        Ok(Value::Object(wrapper))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_schema() -> Map<String, Value> {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        })
        .as_object()
        .cloned()
        .unwrap()
    }

    #[test]
    fn test_mcp_to_tool() {
        let tool = McpTool::new("hello", "desc", test_schema());
        let config = McpCallableToolConfig::default();
        let gemini_tool = mcp_to_tool(&[tool], &config).unwrap();
        let declarations = gemini_tool.function_declarations.unwrap();
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "hello");
        assert!(declarations[0].parameters_json_schema.is_some());
    }

    #[test]
    fn test_mcp_to_tool_duplicate_name() {
        let tool1 = McpTool::new("dup", "desc", test_schema());
        let tool2 = McpTool::new("dup", "desc2", test_schema());
        let config = McpCallableToolConfig::default();
        let result = mcp_to_tool(&[tool1, tool2], &config);
        assert!(matches!(result, Err(Error::InvalidConfig { .. })));
    }

    #[test]
    fn test_append_mcp_usage_header() {
        mark_mcp_tool_usage();
        let mut headers = HeaderMap::new();
        append_mcp_usage_header(&mut headers).unwrap();
        let value = headers
            .get(MCP_USAGE_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap();
        assert_eq!(value, MCP_LABEL);
    }

    #[test]
    fn test_append_mcp_usage_header_with_existing_value() {
        mark_mcp_tool_usage();
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static(MCP_USAGE_HEADER),
            HeaderValue::from_static("google-genai-sdk/1.0.0"),
        );
        append_mcp_usage_header(&mut headers).unwrap();
        let value = headers
            .get(MCP_USAGE_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap();
        assert_eq!(value, "google-genai-sdk/1.0.0 mcp_used/unknown");
    }
}
