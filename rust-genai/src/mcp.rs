//! MCP (Model Context Protocol) integration helpers.
//!
//! This module is experimental and behind the `mcp` feature.

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
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
pub fn set_mcp_usage_header<S: BuildHasher>(headers: &mut HashMap<String, String, S>) {
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

/// 自动在 `HeaderMap` 中追加 MCP 使用标记（仅当检测到 MCP 工具使用）。
///
/// # Errors
///
/// 当 header 值无法写入时返回错误。
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

/// 将 MCP tools 转换为单个 Gemini Tool（包含 `FunctionDeclaration` 列表）。
///
/// # Errors
///
/// 当 MCP 工具存在重复名称时返回错误。
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
        description: tool.description.as_ref().map(ToString::to_string),
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
    /// 创建新的 `McpCallableTool`。
    #[must_use]
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
    ///
    /// # Errors
    ///
    /// 当拉取 MCP 工具失败或工具名称冲突时返回错误。
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
    ///
    /// # Errors
    ///
    /// 当初始化失败或工具转换失败时返回错误。
    pub async fn tool(&mut self) -> Result<Tool> {
        self.initialize().await?;
        mcp_to_tool(&self.mcp_tools, &self.config)
    }

    /// 执行 MCP 工具调用并返回 `FunctionResponse` Parts。
    ///
    /// # Errors
    /// 当初始化失败、调用工具失败或响应解析失败时返回错误。
    pub async fn call_tool(&mut self, function_calls: &[FunctionCall]) -> Result<Vec<Part>> {
        self.initialize().await?;
        let mut parts = Vec::new();
        for call in function_calls {
            let Some(name) = call.name.as_ref() else {
                continue;
            };
            let Some(client) = self.function_name_to_client.get(name) else {
                continue;
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
        Box::pin(async move { Self::tool(self).await })
    }

    fn call_tool(&mut self, function_calls: Vec<FunctionCall>) -> BoxFuture<'_, Result<Vec<Part>>> {
        Box::pin(async move { Self::call_tool(self, &function_calls).await })
    }
}

fn normalize_mcp_arguments(
    args: Option<&Value>,
    function_name: &str,
) -> Result<Option<Map<String, Value>>> {
    match args {
        None | Some(Value::Null) => Ok(None),
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
    use rmcp::model::{
        CallToolResult, ClientInfo, ClientNotification, ClientRequest, ClientResult,
        ListToolsResult, ServerInfo, ServerNotification, ServerRequest, ServerResult,
        Tool as McpTool,
    };
    use rmcp::service::{
        serve_client, serve_server, NotificationContext, Peer, RequestContext, RoleClient,
        RoleServer, RxJsonRpcMessage, Service, ServiceRole, TxJsonRpcMessage,
    };
    use rmcp::transport::Transport;
    use rmcp::ErrorData as McpError;
    use rust_genai_types::content::PartKind;
    use serde_json::json;
    use std::time::Duration;
    use tokio::sync::mpsc;

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

    #[test]
    fn test_set_mcp_usage_header_deduplicates() {
        let mut headers = HashMap::new();
        set_mcp_usage_header(&mut headers);
        set_mcp_usage_header(&mut headers);
        assert_eq!(headers.get(MCP_USAGE_HEADER).unwrap(), MCP_LABEL);
    }

    #[test]
    fn test_normalize_mcp_arguments_variants() {
        let args = normalize_mcp_arguments(None, "tool").unwrap();
        assert!(args.is_none());

        let args = normalize_mcp_arguments(Some(&Value::Null), "tool").unwrap();
        assert!(args.is_none());

        let args = normalize_mcp_arguments(Some(&json!({"k": "v"})), "tool").unwrap();
        let map = args.unwrap();
        assert_eq!(map.get("k").unwrap(), "v");

        let result = normalize_mcp_arguments(Some(&json!(["oops"])), "tool");
        assert!(matches!(result, Err(Error::InvalidConfig { .. })));
    }

    #[test]
    fn test_mcp_result_to_value_wraps_error() {
        let ok_result = CallToolResult::structured(json!({"ok": true}));
        let ok_value = mcp_result_to_value(&ok_result).unwrap();
        assert!(ok_value.get("error").is_none());

        let err_result = CallToolResult::structured_error(json!({"message": "boom"}));
        let err_value = mcp_result_to_value(&err_result).unwrap();
        assert!(err_value.get("error").is_some());
    }

    struct ChannelTransport<R: ServiceRole> {
        tx: mpsc::UnboundedSender<TxJsonRpcMessage<R>>,
        rx: mpsc::UnboundedReceiver<RxJsonRpcMessage<R>>,
    }

    impl<R: ServiceRole> ChannelTransport<R> {
        fn new(
            tx: mpsc::UnboundedSender<TxJsonRpcMessage<R>>,
            rx: mpsc::UnboundedReceiver<RxJsonRpcMessage<R>>,
        ) -> Self {
            Self { tx, rx }
        }
    }

    impl<R: ServiceRole> Transport<R> for ChannelTransport<R> {
        type Error = mpsc::error::SendError<TxJsonRpcMessage<R>>;

        fn send(
            &mut self,
            item: TxJsonRpcMessage<R>,
        ) -> impl std::future::Future<Output = std::result::Result<(), Self::Error>> + Send + 'static
        {
            let tx = self.tx.clone();
            std::future::ready(tx.send(item))
        }

        async fn receive(&mut self) -> Option<RxJsonRpcMessage<R>> {
            self.rx.recv().await
        }

        fn close(
            &mut self,
        ) -> impl std::future::Future<Output = std::result::Result<(), Self::Error>> + Send
        {
            std::future::ready(Ok(()))
        }
    }

    #[derive(Clone)]
    struct TestServer {
        tools: Vec<McpTool>,
    }

    impl TestServer {
        fn new(tools: Vec<McpTool>) -> Self {
            Self { tools }
        }
    }

    impl Service<RoleServer> for TestServer {
        fn handle_request(
            &self,
            request: ClientRequest,
            _context: RequestContext<RoleServer>,
        ) -> impl std::future::Future<Output = std::result::Result<ServerResult, McpError>> + Send + '_
        {
            let tools = self.tools.clone();
            async move {
                match request {
                    ClientRequest::InitializeRequest(_) => {
                        Ok(ServerResult::InitializeResult(ServerInfo::default()))
                    }
                    ClientRequest::ListToolsRequest(_) => Ok(ServerResult::ListToolsResult(
                        ListToolsResult::with_all_items(tools),
                    )),
                    ClientRequest::CallToolRequest(call) => {
                        let name = call.params.name.as_ref();
                        let result = match name {
                            "ok" => CallToolResult::structured(json!({"ok": true})),
                            "fail" => CallToolResult::structured_error(json!({"error": "boom"})),
                            "slow" => {
                                tokio::time::sleep(Duration::from_millis(50)).await;
                                CallToolResult::structured(json!({"slow": true}))
                            }
                            _ => CallToolResult::structured_error(json!({"error": "unknown"})),
                        };
                        Ok(ServerResult::CallToolResult(result))
                    }
                    _ => Ok(ServerResult::empty(())),
                }
            }
        }

        fn handle_notification(
            &self,
            _notification: ClientNotification,
            _context: NotificationContext<RoleServer>,
        ) -> impl std::future::Future<Output = std::result::Result<(), McpError>> + Send + '_
        {
            std::future::ready(Ok(()))
        }

        fn get_info(&self) -> ServerInfo {
            ServerInfo::default()
        }
    }

    #[derive(Clone, Default)]
    struct TestClient;

    impl Service<RoleClient> for TestClient {
        fn handle_request(
            &self,
            _request: ServerRequest,
            _context: RequestContext<RoleClient>,
        ) -> impl std::future::Future<Output = std::result::Result<ClientResult, McpError>> + Send + '_
        {
            std::future::ready(Ok(ClientResult::empty(())))
        }

        fn handle_notification(
            &self,
            _notification: ServerNotification,
            _context: NotificationContext<RoleClient>,
        ) -> impl std::future::Future<Output = std::result::Result<(), McpError>> + Send + '_
        {
            std::future::ready(Ok(()))
        }

        fn get_info(&self) -> ClientInfo {
            ClientInfo::default()
        }
    }

    async fn setup_peer(
        tools: Vec<McpTool>,
    ) -> (
        Peer<RoleClient>,
        rmcp::service::RunningService<RoleClient, TestClient>,
        rmcp::service::RunningService<RoleServer, TestServer>,
    ) {
        let (client_tx, client_rx) = mpsc::unbounded_channel::<TxJsonRpcMessage<RoleClient>>();
        let (server_tx, server_rx) = mpsc::unbounded_channel::<TxJsonRpcMessage<RoleServer>>();

        let client_transport = ChannelTransport::new(client_tx, server_rx);
        let server_transport = ChannelTransport::new(server_tx, client_rx);

        let server_task =
            tokio::spawn(
                async move { serve_server(TestServer::new(tools), server_transport).await },
            );

        let client_service = serve_client(TestClient, client_transport).await.unwrap();
        let server_service = server_task.await.unwrap().unwrap();
        let peer = client_service.peer().clone();
        (peer, client_service, server_service)
    }

    #[tokio::test]
    async fn test_mcp_callable_tool_roundtrip() {
        let tools = vec![McpTool::new("ok", "desc", test_schema())];
        let (peer, _client_service, _server_service) = setup_peer(tools).await;

        let mut tool = McpCallableTool::new(vec![peer], McpCallableToolConfig::default());
        let gemini_tool = tool.tool().await.unwrap();
        assert!(gemini_tool.function_declarations.is_some());

        let calls = vec![FunctionCall {
            id: Some("call-1".into()),
            name: Some("ok".into()),
            args: Some(json!({"a": 1})),
            partial_args: None,
            will_continue: None,
        }];

        let parts = tool.call_tool(&calls).await.unwrap();
        assert_eq!(parts.len(), 1);
        match &parts[0].kind {
            PartKind::FunctionResponse { function_response } => {
                assert_eq!(function_response.name.as_deref(), Some("ok"));
                let response = function_response.response.as_ref().unwrap();
                let structured = response.get("structuredContent").unwrap();
                assert!(structured.get("ok").is_some());
            }
            _ => panic!("expected function response part"),
        }
    }

    #[tokio::test]
    async fn test_mcp_callable_tool_error_and_timeout() {
        let tools = vec![
            McpTool::new("fail", "desc", test_schema()),
            McpTool::new("slow", "desc", test_schema()),
        ];
        let (peer, _client_service, _server_service) = setup_peer(tools).await;

        let config = McpCallableToolConfig {
            timeout: Some(Duration::from_millis(5)),
            behavior: None,
        };
        let mut tool = McpCallableTool::new(vec![peer], config);

        let calls = vec![FunctionCall {
            id: Some("call-2".into()),
            name: Some("fail".into()),
            args: None,
            partial_args: None,
            will_continue: None,
        }];
        let parts = tool.call_tool(&calls).await.unwrap();
        match &parts[0].kind {
            PartKind::FunctionResponse { function_response } => {
                let response = function_response.response.as_ref().unwrap();
                assert!(response.get("error").is_some());
            }
            _ => panic!("expected function response part"),
        }

        let timeout_calls = vec![FunctionCall {
            id: Some("call-3".into()),
            name: Some("slow".into()),
            args: Some(json!({})),
            partial_args: None,
            will_continue: None,
        }];
        let timeout_result = tool.call_tool(&timeout_calls).await;
        assert!(matches!(timeout_result, Err(Error::Timeout { .. })));
    }
}
