//! Automatic Function Calling (AFC) helpers.

use std::collections::HashMap;
use std::future::Future;
use std::hash::BuildHasher;

use futures_util::future::BoxFuture;
use rust_genai_types::content::{FunctionCall, FunctionResponse, Part};
use rust_genai_types::models::GenerateContentConfig;
use rust_genai_types::tool::{FunctionDeclaration, Tool};
use serde_json::Value;

use crate::error::{Error, Result};

/// 默认最大远程调用次数。
pub const DEFAULT_MAX_REMOTE_CALLS: usize = 10;

/// 可调用工具接口。
pub trait CallableTool: Send {
    fn tool(&mut self) -> BoxFuture<'_, Result<Tool>>;
    fn call_tool(&mut self, function_calls: Vec<FunctionCall>) -> BoxFuture<'_, Result<Vec<Part>>>;
}

/// Inline callable tool handler 类型。
pub type ToolHandler =
    Box<dyn Fn(Value) -> BoxFuture<'static, Result<Value>> + Send + Sync + 'static>;

/// 以函数声明 + handler 组合的可调用工具。
#[derive(Default)]
pub struct InlineCallableTool {
    tool: Tool,
    handlers: HashMap<String, ToolHandler>,
}

impl InlineCallableTool {
    /// 通过 `FunctionDeclaration` 列表创建工具。
    #[must_use]
    pub fn from_declarations(declarations: Vec<FunctionDeclaration>) -> Self {
        Self {
            tool: Tool {
                function_declarations: Some(declarations),
                ..Tool::default()
            },
            handlers: HashMap::new(),
        }
    }

    /// 注册 handler。
    pub fn register_handler<F, Fut>(&mut self, name: impl Into<String>, handler: F)
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value>> + Send + 'static,
    {
        let key = name.into();
        self.handlers.insert(
            key,
            Box::new(move |value| {
                let fut = handler(value);
                Box::pin(fut)
            }),
        );
    }

    /// 使用 builder 风格注册 handler。
    #[must_use]
    pub fn with_handler<F, Fut>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value>> + Send + 'static,
    {
        self.register_handler(name, handler);
        self
    }
}

impl CallableTool for InlineCallableTool {
    fn tool(&mut self) -> BoxFuture<'_, Result<Tool>> {
        Box::pin(async move { Ok(self.tool.clone()) })
    }

    fn call_tool(&mut self, function_calls: Vec<FunctionCall>) -> BoxFuture<'_, Result<Vec<Part>>> {
        Box::pin(async move {
            let mut parts = Vec::new();
            for call in function_calls {
                let Some(name) = call.name.as_ref() else {
                    continue;
                };
                let Some(handler) = self.handlers.get(name) else {
                    continue;
                };
                let args = call.args.clone().unwrap_or(Value::Null);
                let response_value = handler(args).await?;
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
        })
    }
}

/// 解析 callable tools，返回声明列表与函数映射。
///
/// # Errors
/// 当工具声明重复或工具返回错误时返回错误。
pub async fn resolve_callable_tools(
    callable_tools: &mut [Box<dyn CallableTool>],
) -> Result<CallableToolInfo> {
    let mut tools = Vec::new();
    let mut function_map: HashMap<String, usize> = HashMap::new();

    for (index, tool) in callable_tools.iter_mut().enumerate() {
        let declaration_tool = tool.tool().await?;
        if let Some(declarations) = &declaration_tool.function_declarations {
            for declaration in declarations {
                if function_map.contains_key(&declaration.name) {
                    return Err(Error::InvalidConfig {
                        message: format!("Duplicate tool declaration name: {}", declaration.name),
                    });
                }
                function_map.insert(declaration.name.clone(), index);
            }
        }
        tools.push(declaration_tool);
    }

    Ok(CallableToolInfo {
        tools,
        function_map,
    })
}

/// 调用 callable tools。
///
/// # Errors
/// 当函数调用缺少工具或工具调用失败时返回错误。
pub async fn call_callable_tools<S: BuildHasher + Sync>(
    callable_tools: &mut [Box<dyn CallableTool>],
    function_map: &HashMap<String, usize, S>,
    function_calls: &[FunctionCall],
) -> Result<Vec<Part>> {
    let mut grouped: HashMap<usize, Vec<FunctionCall>> = HashMap::new();
    for call in function_calls {
        let name = call.name.as_ref().ok_or_else(|| Error::InvalidConfig {
            message: "Function call name was not returned by the model.".into(),
        })?;
        let index = function_map.get(name).ok_or_else(|| Error::InvalidConfig {
            message: format!(
                "Automatic function calling was requested, but not all the tools the model used implement the CallableTool interface. Missing tool: {name}."
            ),
        })?;
        grouped.entry(*index).or_default().push(call.clone());
    }

    let mut parts = Vec::new();
    for (index, calls) in grouped {
        let response_parts = callable_tools[index].call_tool(calls).await?;
        parts.extend(response_parts);
    }
    Ok(parts)
}

/// callable tools 解析结果。
pub struct CallableToolInfo<S = std::collections::hash_map::RandomState> {
    pub tools: Vec<Tool>,
    pub function_map: HashMap<String, usize, S>,
}

/// 判断是否应禁用 AFC。
#[must_use]
pub fn should_disable_afc(config: &GenerateContentConfig, has_callable_tools: bool) -> bool {
    if !has_callable_tools {
        return true;
    }
    if config
        .automatic_function_calling
        .as_ref()
        .and_then(|cfg| cfg.disable)
        .unwrap_or(false)
    {
        return true;
    }
    if let Some(max_calls) = config
        .automatic_function_calling
        .as_ref()
        .and_then(|cfg| cfg.maximum_remote_calls)
    {
        if max_calls <= 0 {
            return true;
        }
    }
    false
}

/// 获取最大远程调用次数。
#[must_use]
pub fn max_remote_calls(config: &GenerateContentConfig) -> usize {
    config
        .automatic_function_calling
        .as_ref()
        .and_then(|cfg| cfg.maximum_remote_calls)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_MAX_REMOTE_CALLS)
}

/// 是否应附加 AFC 历史。
#[must_use]
pub fn should_append_history(config: &GenerateContentConfig) -> bool {
    !config
        .automatic_function_calling
        .as_ref()
        .and_then(|cfg| cfg.ignore_call_history)
        .unwrap_or(false)
}

/// 检查 AFC 兼容性（禁止未实现 `CallableTool` 的 function declarations）。
///
/// # Errors
/// 当发现不兼容工具时返回错误。
pub fn validate_afc_tools<S: BuildHasher>(
    _callable_function_map: &HashMap<String, usize, S>,
    tools: Option<&[Tool]>,
) -> Result<()> {
    let Some(tools) = tools else {
        return Ok(());
    };

    for tool in tools {
        if let Some(declarations) = &tool.function_declarations {
            if !declarations.is_empty() {
                return Err(Error::InvalidConfig {
                    message: "Incompatible tools found. Automatic function calling does not support mixing CallableTools with basic function declarations.".into(),
                });
            }
        }
    }
    Ok(())
}

/// 校验 AFC 与其他配置的冲突。
///
/// # Errors
/// 当检测到不兼容配置时返回错误。
pub fn validate_afc_config(config: &GenerateContentConfig) -> Result<()> {
    if config
        .tool_config
        .as_ref()
        .and_then(|cfg| cfg.function_calling_config.as_ref())
        .and_then(|cfg| cfg.stream_function_call_arguments)
        .unwrap_or(false)
        && !config
            .automatic_function_calling
            .as_ref()
            .and_then(|cfg| cfg.disable)
            .unwrap_or(false)
    {
        return Err(Error::InvalidConfig {
            message: "stream_function_call_arguments is not compatible with automatic function calling. Disable AFC or disable stream_function_call_arguments.".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_genai_types::models::AutomaticFunctionCallingConfig;
    use rust_genai_types::tool::{FunctionDeclaration, Tool};
    use serde_json::json;

    #[test]
    fn test_should_disable_afc_when_max_calls_zero() {
        let config = GenerateContentConfig {
            automatic_function_calling: Some(AutomaticFunctionCallingConfig {
                maximum_remote_calls: Some(0),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(should_disable_afc(&config, true));
    }

    #[test]
    fn test_should_append_history_respects_ignore_flag() {
        let config = GenerateContentConfig {
            automatic_function_calling: Some(AutomaticFunctionCallingConfig {
                ignore_call_history: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(!should_append_history(&config));
    }

    #[test]
    fn test_validate_afc_tools_rejects_plain_declarations() {
        let tool = Tool {
            function_declarations: Some(vec![FunctionDeclaration {
                name: "test_fn".to_string(),
                description: None,
                parameters: None,
                parameters_json_schema: None,
                response: None,
                response_json_schema: None,
                behavior: None,
            }]),
            ..Default::default()
        };
        let err = validate_afc_tools(&HashMap::new(), Some(&[tool])).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[tokio::test]
    async fn test_inline_callable_tool_roundtrip() {
        let mut tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
            name: "sum".to_string(),
            description: None,
            parameters: None,
            parameters_json_schema: None,
            response: None,
            response_json_schema: None,
            behavior: None,
        }]);
        tool.register_handler("sum", |value| async move {
            let a = value["a"].as_i64().unwrap_or(0);
            let b = value["b"].as_i64().unwrap_or(0);
            Ok(json!({ "result": a + b }))
        });

        let mut tools: Vec<Box<dyn CallableTool>> = vec![Box::new(tool)];
        let info = resolve_callable_tools(&mut tools).await.unwrap();
        assert!(info.function_map.contains_key("sum"));

        let calls = vec![FunctionCall {
            id: Some("call-1".into()),
            name: Some("sum".into()),
            args: Some(json!({"a": 1, "b": 2})),
            partial_args: None,
            will_continue: None,
        }];
        let parts = call_callable_tools(&mut tools, &info.function_map, &calls)
            .await
            .unwrap();
        assert_eq!(parts.len(), 1);
    }

    #[tokio::test]
    async fn test_call_callable_tools_rejects_missing_name() {
        let mut tools: Vec<Box<dyn CallableTool>> = Vec::new();
        let calls = vec![FunctionCall {
            id: None,
            name: None,
            args: None,
            partial_args: None,
            will_continue: None,
        }];
        let err = call_callable_tools(&mut tools, &HashMap::new(), &calls)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[tokio::test]
    async fn test_call_callable_tools_rejects_unknown_tool() {
        let calls = vec![FunctionCall {
            id: Some("call-1".into()),
            name: Some("missing".into()),
            args: None,
            partial_args: None,
            will_continue: None,
        }];
        let err = call_callable_tools(&mut [], &HashMap::new(), &calls)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }
}
