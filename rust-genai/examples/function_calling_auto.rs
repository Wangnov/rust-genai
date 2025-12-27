use rust_genai::afc::InlineCallableTool;
use rust_genai::types::content::Content;
use rust_genai::types::enums::FunctionCallingMode;
use rust_genai::types::models::{AutomaticFunctionCallingConfig, GenerateContentConfig};
use rust_genai::types::tool::{FunctionCallingConfig, FunctionDeclaration, Schema, ToolConfig};
use serde_json::json;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let mut add_tool = InlineCallableTool::from_declarations(vec![FunctionDeclaration {
        name: "add_numbers".into(),
        description: Some("Add two numbers".into()),
        parameters: Some(
            Schema::object()
                .property("a", Schema::number())
                .property("b", Schema::number())
                .required("a")
                .required("b")
                .build(),
        ),
        parameters_json_schema: None,
        response: None,
        response_json_schema: None,
        behavior: None,
    }]);

    add_tool.register_handler("add_numbers", |args| async move {
        let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
        Ok(json!({ "result": a + b }))
    });

    let client = rust_genai::Client::from_env()?;
    let config = GenerateContentConfig {
        tool_config: Some(ToolConfig {
            function_calling_config: Some(FunctionCallingConfig {
                // Auto 允许模型在调用工具后给出最终文本结果。
                mode: Some(FunctionCallingMode::Auto),
                ..FunctionCallingConfig::default()
            }),
            ..ToolConfig::default()
        }),
        automatic_function_calling: Some(AutomaticFunctionCallingConfig {
            maximum_remote_calls: Some(5),
            ..AutomaticFunctionCallingConfig::default()
        }),
        ..GenerateContentConfig::default()
    };

    let response = client
        .models()
        .generate_content_with_callable_tools(
            "gemini-2.5-flash",
            vec![Content::text("计算 2.5 + 3.1")],
            config,
            vec![Box::new(add_tool)],
        )
        .await?;

    println!("{:?}", response.text());
    Ok(())
}
