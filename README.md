# Rust Gemini SDK (rust-genai)

[![crates.io](https://img.shields.io/crates/v/rust-genai.svg)](https://crates.io/crates/rust-genai)
[![docs.rs](https://docs.rs/rust-genai/badge.svg)](https://docs.rs/rust-genai)
[![license](https://img.shields.io/crates/l/rust-genai.svg)](LICENSE)

**Language: 中文 | [English](#installation-1)**

用于 Google Gemini API 和 Vertex AI 的 Rust SDK。本项目致力于与官方 Go、JavaScript、Python SDK 保持功能对齐，同时充分利用 Rust 的安全性和性能优势。

## 功能特性

- 基于 Tokio 的异步优先客户端
- 支持 Gemini API 和 Vertex AI 后端
- Models、Chats、Files、Caches、Batches、Operations API
- 流式响应（SSE）
- 函数调用和工具系统
- 自动函数调用（AFC）与可调用工具
- Live API、会话恢复、Live Music（实验性）
- Interactions API（Beta）和 Deep Research（Preview）
- Grounding 元数据、logprobs、媒体分辨率设置
- Count/Compute Tokens (Vertex AI) + 可选本地估算（feature: `kitoken`）
- Recontext / Segment Image (Vertex AI)
- MCP 支持（feature: `mcp`，实验性）

## 最低支持 Rust 版本（MSRV）

| Feature | MSRV |
|---------|------|
| 默认（无 `mcp`） | Rust 1.88+ |
| 启用 `mcp` feature | Rust 1.88+ |

当前 MSRV 为 Rust 1.88+；`mcp` feature 同样需要 1.88+（上游依赖 `rmcp` → `darling`）。

## 安装

```toml
[dependencies]
rust-genai = "0.1.0"
```

## 快速开始

```rust
use rust_genai::Client;
use rust_genai::types::content::Content;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let response = client
        .models()
        .generate_content("gemini-2.5-flash", vec![Content::text("你好，Rust!")])
        .await?;
    println!("{response:?}");
    Ok(())
}
```

## 文档

- 快速开始：`docs/getting-started.md`
- 最佳实践：`docs/best-practices.md`
- MCP：`docs/mcp.md`
- API 文档：`cargo doc --open`

## 示例

查看 `rust-genai/examples/` 目录，涵盖核心功能、错误处理、性能与 Live Music。

## 工作区结构

- `rust-genai`：主客户端 crate（公共 API）
- `rust-genai-types`：共享类型定义
- `rust-genai-macros`：过程宏

## 许可证

Apache-2.0。详见 `LICENSE`。

---

<a id="installation-1"></a>

**Language: [中文](#rust-gemini-sdk-rust-genai) | English**

Rust SDK for the Google Gemini API and Vertex AI. This workspace aims to stay feature-aligned with the official Go, JavaScript, and Python SDKs while taking advantage of Rust's safety and performance.

## Features

- Async-first client with Tokio
- Gemini API and Vertex AI backends
- Models, Chats, Files, Caches, Batches, Operations
- Streaming (SSE)
- Function calling and tool system
- Automatic function calling (AFC) with callable tools
- Live API, session resumption, and Live Music (experimental)
- Interactions API (Beta) and Deep Research (Preview)
- Grounding metadata, logprobs, and media resolution
- Count/Compute Tokens (Vertex AI) + optional local estimation (feature: `kitoken`)
- Recontext / Segment Image (Vertex AI)
- MCP support (feature: `mcp`, experimental)

## MSRV (Minimum Supported Rust Version)

| Feature | MSRV |
|---------|------|
| Default (no `mcp`) | Rust 1.88+ |
| With `mcp` feature | Rust 1.88+ |

Current MSRV is Rust 1.88+; the `mcp` feature also requires 1.88+ due to upstream dependencies (`rmcp` → `darling`).

## Installation

```toml
[dependencies]
rust-genai = "0.1.0"
```

## Quickstart

```rust
use rust_genai::Client;
use rust_genai::types::content::Content;

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let response = client
        .models()
        .generate_content("gemini-2.5-flash", vec![Content::text("Hello from Rust!")])
        .await?;
    println!("{response:?}");
    Ok(())
}
```

## Documentation

- Getting Started: `docs/getting-started.md`
- Best Practices: `docs/best-practices.md`
- MCP: `docs/mcp.md`
- API Reference: `cargo doc --open`

## Examples

See `rust-genai/examples/` for core features, error handling, performance, and Live Music.

## Workspace Layout

- `rust-genai`: main client crate (public API)
- `rust-genai-types`: shared type definitions
- `rust-genai-macros`: procedural macros

## License

Apache-2.0. See `LICENSE`.
