# 最佳实践

## 1. 使用结构化 Content

优先使用 `Content` 与 `Part` 构建结构化输入，避免手写 JSON。

```rust
use rust_genai::types::content::{Content, Part};
let content = Content::new_user(vec![
    Part::text("请总结下面的内容"),
]);
```

## 2. 明确错误处理

所有 API 返回 `Result<T, rust_genai::Error>`，建议统一处理：

```rust
if let Err(err) = client.models().list().await {
    eprintln!("request failed: {err}");
}
```

## 3. 控制请求超时与代理

```rust
let client = rust_genai::Client::builder()
    .api_key("...")
    .timeout(30)
    .proxy("http://localhost:7890")
    .build()?;
```

## 4. 避免重复初始化客户端

`Client` 内部使用 `Arc` 与连接复用，建议全局共享：

```rust
let client = std::sync::Arc::new(rust_genai::Client::from_env()?);
```

## 5. 流式输出的性能

`generate_content_stream` 使用 SSE 解析，适合大输出或低延迟场景。
如果只需要一次性结果，使用 `generate_content`。

## 6. Live API

Live API 是长连接，确保在使用完毕后调用 `close()` 释放资源：

```rust
let session = client.live().connect(model, Default::default()).await?;
session.close().await?;
```

## 7. 日志与调试

建议在生产环境中对错误日志进行分级，并避免输出敏感信息。

## 8. 本地 Token 估算（可选）

SDK 提供本地估算接口（非精确计费），适合在请求前做粗略预算。
如需更高精度，可启用 `kitoken` feature 并加载 SentencePiece 模型：

```rust
use rust_genai::tokenizer::{SimpleTokenEstimator, TokenEstimator};
#[cfg(feature = "kitoken")]
use rust_genai::tokenizer::kitoken::KitokenEstimator;

let estimator = SimpleTokenEstimator::default();
let estimate = estimator.estimate_tokens(&contents);
println!("estimated tokens: {estimate}");

// 可选：SentencePiece 模型估算
#[cfg(feature = "kitoken")]
let spm_estimator = KitokenEstimator::from_sentencepiece_file("path/to/model.model")?;

// 或者按模型名自动下载（异步）
#[cfg(feature = "kitoken")]
let spm_estimator = KitokenEstimator::from_model_name("gemini-2.5-flash").await?;

// 可选：本地 compute_tokens（文本 / 函数调用 / 函数响应 / 代码执行）
#[cfg(feature = "kitoken")]
let local_tokens = spm_estimator.compute_tokens(&contents)?;
println!("local tokens: {local_tokens:?}");
```

> 注意：本地估算仅供参考，最终计费以服务端 `countTokens` 为准。
> 提示：`kitoken` 会在本地缓存模型文件，需要网络下载模型（第一次）。
> 提示：`compute_tokens` 返回的 token 字节使用 base64 编码。
> 提示：本地 `compute_tokens` 不支持图片/文件/二进制内容（会直接返回错误）。

## 9. 自动函数调用（AFC）

当你希望 SDK 自动执行函数调用时，使用 `generate_content_with_callable_tools`：

```rust
use rust_genai::afc::InlineCallableTool;
use rust_genai::types::content::Content;
use rust_genai::types::models::{AutomaticFunctionCallingConfig, GenerateContentConfig};

let mut tool = InlineCallableTool::from_declarations(vec![
    /* FunctionDeclaration 列表 */
]);
tool.register_handler("tool_name", |args| async move {
    Ok(serde_json::json!({ "ok": true, "input": args }))
});

let config = GenerateContentConfig {
    automatic_function_calling: Some(AutomaticFunctionCallingConfig {
        maximum_remote_calls: Some(5),
        ..Default::default()
    }),
    ..Default::default()
};

let response = client
    .models()
    .generate_content_with_callable_tools(
        "gemini-2.5-flash",
        vec![Content::text("调用工具")],
        config,
        vec![Box::new(tool)],
    )
    .await?;
```

## 10. 模型管理（更新 / 删除）

对已存在的 tuned model，可更新显示名/描述或删除资源（谨慎操作）：

```rust
use rust_genai::types::models::{DeleteModelConfig, UpdateModelConfig};

let updated = client
    .models()
    .update(
        "projects/your-project/locations/us-central1/models/your-model",
        UpdateModelConfig {
            display_name: Some("new-display-name".to_string()),
            description: Some("updated description".to_string()),
            ..Default::default()
        },
    )
    .await?;
println!("updated model: {:?}", updated.name);

let _ = client
    .models()
    .delete(
        "projects/your-project/locations/us-central1/models/your-model",
        DeleteModelConfig::default(),
    )
    .await?;
```

## 11. 自动分页 all()

常见列表接口支持 `all()` 自动翻页，适合需要一次性拉取所有资源：

```rust
let models = client.models().all().await?;
let files = client.files().all().await?;
let caches = client.caches().all().await?;
```

## 12. 聊天兼容别名

`ChatSession` 提供 `send` / `send_stream` 作为 `send_message` 的别名：

```rust
let chat = client.chats().create("gemini-2.5-flash");
let _ = chat.send("hello").await?;
let _stream = chat.send_stream("streaming hello").await?;
```

## 13. Vertex-only 图像能力

`recontext_image` 与 `segment_image` 仅支持 Vertex AI：

```rust
use rust_genai::Client;
use rust_genai::types::models::{Image, RecontextImageConfig, RecontextImageSource};

let client = Client::new_vertex("my-project", "us-central1")?;
let source = RecontextImageSource {
    prompt: Some("studio product photo".to_string()),
    product_images: Some(vec![rust_genai::types::models::ProductImage {
        product_image: Some(Image {
            gcs_uri: Some("gs://bucket/product.jpg".to_string()),
            ..Default::default()
        }),
    }]),
    ..Default::default()
};
let _ = client
    .models()
    .recontext_image("imagen-product-recontext-preview-06-30", source, RecontextImageConfig::default())
    .await?;
```

## 14. Retrieval / Vertex RAG

Vertex RAG Store 仅在 Vertex AI 后端可用：

```rust
use rust_genai::types::tool::{Retrieval, Tool, VertexRagStore, VertexRagStoreRagResource};

let tool = Tool {
    retrieval: Some(Retrieval {
        vertex_rag_store: Some(VertexRagStore {
            rag_resources: Some(vec![VertexRagStoreRagResource {
                rag_corpus: Some("projects/xxx/locations/us/ragCorpora/yyy".to_string()),
                rag_file_ids: None,
            }]),
            ..Default::default()
        }),
        ..Default::default()
    }),
    ..Default::default()
};
```
