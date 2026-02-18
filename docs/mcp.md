# MCP 支持（基于 rmcp）

本 SDK 已提供 MCP（Model Context Protocol）实验性支持，可将 MCP 工具接入
Gemini 的函数调用系统。实现基于官方 Rust SDK `rmcp`。

## 依赖与特性

- **Feature**：`mcp`
- **依赖**：`rmcp = 0.12.0`（MIT，官方 Rust SDK）

启用方式：

```toml
rust-genai = { version = "0.1.1", features = ["mcp"] }
```

## 使用方式

1. 使用 `rmcp` 创建 MCP client（stdio 示例）
2. 使用 `McpCallableTool` 获取 Gemini Tool
3. 将 Tool 放入 `GenerateContentConfig.tools`
4. 收到 function_call 后，用 `McpCallableTool::call_tool` 生成 function_response 并继续对话

或使用自动函数调用（AFC）：

```rust
let response = client
    .models()
    .generate_content_with_callable_tools(
        "gemini-2.5-flash",
        vec![Content::text("执行 MCP 工具")],
        GenerateContentConfig::default(),
        vec![Box::new(mcp_tool)],
    )
    .await?;
```

示例（简化版）：

```rust
use rust_genai::mcp::{McpCallableTool, McpCallableToolConfig};
use rust_genai::types::content::{Content, Role};
use rust_genai::types::models::GenerateContentConfig;
use rmcp::service::ServiceExt;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use tokio::process::Command;
let transport = TokioChildProcess::new(Command::new("npx").configure(|cmd| {
    cmd.arg("-y").arg("@modelcontextprotocol/server-everything");
}))?;
let service = ().serve(transport).await?;
let peer = service.peer().clone();

let mut mcp_tool = McpCallableTool::new(vec![peer], McpCallableToolConfig::default());
let tool = mcp_tool.tool().await?;

let client = rust_genai::Client::from_env()?;
let config = GenerateContentConfig {
    tools: Some(vec![tool]),
    ..GenerateContentConfig::default()
};
let response = client
    .models()
    .generate_content_with_config("gemini-2.5-flash", vec![Content::text("列出 git 状态")], config)
    .await?;

let function_calls: Vec<_> = response.function_calls().into_iter().cloned().collect();
let parts = mcp_tool.call_tool(&function_calls).await?;

let followup = client
    .models()
    .generate_content_with_config(
        "gemini-2.5-flash",
        vec![Content::from_parts(parts, Role::Function)],
        GenerateContentConfig::default(),
    )
    .await?;
```

## 观测标记（可选）

SDK 会在检测到 MCP 工具使用后，自动为请求追加 `x-goog-api-client` 的
`mcp_used/unknown` 标记。也可以手动指定：

```rust
use rust_genai::mcp::set_mcp_usage_header;
use std::collections::HashMap;

let mut headers = HashMap::new();
set_mcp_usage_header(&mut headers);
// 然后通过 ClientBuilder::header(...) 写入 x-goog-api-client
```

## 行为说明

- `mcp_to_tool` 会将 MCP tools 转换为 Gemini `Tool`，并对重复名称做冲突检测。
- `McpCallableTool` 会缓存 MCP tool 列表并建立函数名映射。
- `call_tool` 仅接受 `function_call.args` 为 JSON object；否则返回参数错误。
- MCP 返回内容会被序列化到 `FunctionResponse.response`（若 `is_error=true` 则包裹在 `{"error": ...}` 中）。
- 可选：启用 `rust-genai-types` 的 `mcp` feature 后，可用 `FunctionResponse::from_mcp_response` 快速构造响应。

## 限制

- MCP 仍处于快速迭代期，`rmcp` 版本已锁定，升级需谨慎验证。
- MCP 的 JSON Schema 与 Gemini 的 JSON Schema 语义可能存在差异，必要时请做校验或过滤。
