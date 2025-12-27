#[cfg(not(feature = "mcp"))]
fn main() {
    eprintln!("此示例需要启用 mcp feature：cargo run --example mcp_basic --features mcp");
}

#[cfg(feature = "mcp")]
use rmcp::service::ServiceExt;
#[cfg(feature = "mcp")]
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
#[cfg(feature = "mcp")]
use rust_genai::mcp::{McpCallableTool, McpCallableToolConfig};
#[cfg(feature = "mcp")]
use rust_genai::types::content::{Content, Role};
#[cfg(feature = "mcp")]
use rust_genai::types::models::GenerateContentConfig;
#[cfg(feature = "mcp")]
use tokio::process::Command;

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    // 启动 MCP stdio server（示例使用 npm 包）
    let transport = TokioChildProcess::new(Command::new("npx").configure(|cmd| {
        cmd.arg("-y").arg("@modelcontextprotocol/server-everything");
    }))?;
    let service =
        ().serve(transport)
            .await
            .map_err(|err| rust_genai::Error::InvalidConfig {
                message: format!("MCP 初始化失败: {err}"),
            })?;
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
        .generate_content_with_config(
            "gemini-2.5-flash",
            vec![Content::text("列出仓库的 git 状态")],
            config,
        )
        .await?;

    let function_calls: Vec<_> = response.function_calls().into_iter().cloned().collect();
    let parts = mcp_tool.call_tool(&function_calls).await?;
    if parts.is_empty() {
        println!("模型未触发 MCP 工具调用。");
        return Ok(());
    }

    let followup = client
        .models()
        .generate_content_with_config(
            "gemini-2.5-flash",
            vec![Content::from_parts(parts, Role::Function)],
            GenerateContentConfig::default(),
        )
        .await?;

    println!("{:?}", followup.text());
    Ok(())
}
