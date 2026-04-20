# Getting Started

本指南介绍如何在 Rust 项目中使用 `rust-genai` SDK 访问 Gemini API 与 Vertex AI。

## 1. 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
rust-genai = "0.3.1"
tokio = { version = "1", features = ["full"] }
```

## 2. 配置 API Key

推荐通过环境变量设置：

```bash
export GEMINI_API_KEY="YOUR_API_KEY"
# 或
export GOOGLE_API_KEY="YOUR_API_KEY"
```

Vertex AI 可直接使用官方环境变量：

```bash
export GOOGLE_GENAI_USE_VERTEXAI=true
export GOOGLE_CLOUD_PROJECT="my-project"
export GOOGLE_CLOUD_LOCATION="us-central1"
export GOOGLE_GENAI_API_VERSION="v1"
```

## 3. 发送首个请求

```rust
use rust_genai::Client;
use rust_genai::types::content::Content;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let response = client
        .models()
        .generate_content("gemini-2.5-flash-lite", vec![Content::text("你好，Rust!")])
        .await?;

    println!("{:?}", response.text());
    Ok(())
}
```

## 4. 流式响应（SSE）

```rust
use futures_util::StreamExt;
use rust_genai::Client;
use rust_genai::types::content::Content;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let mut stream = client
        .models()
        .generate_content_stream(
            "gemini-2.5-flash-lite",
            vec![Content::text("用三句话介绍 Rust")],
            Default::default(),
        )
        .await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(text) = chunk.text() {
            print!("{text}");
        }
    }
    Ok(())
}
```

## 5. Vertex AI

```rust
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::new_vertex("my-project", "us-central1")?;
    let models = client.models().list().await?;
    println!("{:?}", models.models);
    Ok(())
}
```

同样可以直接使用 `Client::from_env()` 读取上面的官方 Vertex 环境变量。

## 6. OAuth / ADC

在使用 ADC 之前，请先配置 Application Default Credentials（例如执行 `gcloud auth application-default login`，或设置 `GOOGLE_APPLICATION_CREDENTIALS` 指向服务账号 JSON）。

```rust
use rust_genai::Client;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::with_adc()?;
    let models = client.models().list().await?;
    println!("{:?}", models.models);
    Ok(())
}
```

更多完整示例请见 `rust-genai/examples/` 目录。

更多配置细节可查看：

- `docs/auth.md`
- `docs/error-handling.md`
- `docs/retry-timeout.md`

## 7. API 版本选择

SDK 默认使用带预览功能覆盖面的 API 版本：

- Gemini Developer API: `v1beta`
- Vertex AI: `v1beta1`

如果你希望手动切换版本，请查看 `docs/api-versions.md`。
