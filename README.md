# Rust Gemini SDK (rust-genai)

[![crates.io](https://img.shields.io/crates/v/rust-genai.svg)](https://crates.io/crates/rust-genai)
[![docs.rs](https://img.shields.io/docsrs/rust-genai)](https://docs.rs/rust-genai)
[![license](https://img.shields.io/github/license/Wangnov/rust-genai)](LICENSE)
[![CI](https://github.com/Wangnov/rust-genai/actions/workflows/ci.yml/badge.svg)](https://github.com/Wangnov/rust-genai/actions/workflows/ci.yml)
[![Crates.io Downloads](https://img.shields.io/crates/d/rust-genai.svg)](https://crates.io/crates/rust-genai)
[![MSRV](https://img.shields.io/crates/msrv/rust-genai)](https://crates.io/crates/rust-genai)
[![GitHub Release](https://img.shields.io/github/v/release/Wangnov/rust-genai)](https://github.com/Wangnov/rust-genai/releases)
[![Deps](https://deps.rs/repo/github/Wangnov/rust-genai/status.svg)](https://deps.rs/repo/github/Wangnov/rust-genai)
[![codecov](https://codecov.io/gh/wangnov/rust-genai/branch/main/graph/badge.svg)](https://codecov.io/gh/wangnov/rust-genai)

**Language: 中文 | [English](#installation-1)**

> 社区维护的 Rust SDK，用于 Google Gemini API 和 Vertex AI。
> Google 维护官方 Go、JavaScript、Python SDK；本仓库提供社区维护的 Rust 实现。

用于 Google Gemini API 和 Vertex AI 的 Rust SDK。本项目致力于与官方 Go、JavaScript、Python SDK 保持功能对齐，同时充分利用 Rust 的安全性和性能优势。

## 功能特性

- 基于 Tokio 的异步优先客户端
- 支持 Gemini API 和 Vertex AI 后端
- Models、Chats、Files、Caches、Batches、Operations API
- 默认自动重试 `408`、`429`、`500`、`502`、`503`、`504`
- 流式响应（SSE）
- 结构化 JSON 生成辅助（`generate_json`）
- 事件级流式辅助（`generate_content_event_stream`）
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
rust-genai = "0.3.1"
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
        .generate_content("gemini-2.5-flash-lite", vec![Content::text("你好，Rust!")])
        .await?;
    println!("{response:?}");
    Ok(())
}
```

## 文档

- 快速开始：`docs/getting-started.md`
- 认证指南：`docs/auth.md`
- 兼容矩阵：`docs/compatibility-matrix.md`
- API 版本：`docs/api-versions.md`
- 错误处理：`docs/error-handling.md`
- Retry / Timeout：`docs/retry-timeout.md`
- 版本与稳定性：`docs/versioning.md`
- LLM 代码生成说明：`llms.txt`
- 官方来源清单：`docs/official-sources.md`
- 规范同步：`docs/spec-sync.md`
- 最佳实践：`docs/best-practices.md`
- MCP：`docs/mcp.md`
- 发布流程：`docs/release.md`
- API 文档：`cargo doc --open`

## 示例

查看 `rust-genai/examples/` 目录，涵盖核心功能、错误处理、性能与 Live Music。

## 稳定性分层

| Surface | Status | Scope |
|---------|--------|-------|
| Client、Models、Chats、Files、Caches、Batches、Operations、Tokens、Embeddings、SSE Streaming | Stable | 主线能力，遵循语义化版本演进 |
| Interactions API | Beta | 接口跟随官方快速演进 |
| Deep Research | Preview | 适合前沿工作流验证 |
| Live Music | Experimental | 适合单独评估后接入 |
| MCP (`feature = "mcp"`) | Experimental | 通过 feature gate 暴露，适合扩展集成 |

详细说明见 `docs/versioning.md`。

## 工作区结构

- `rust-genai`：主客户端 crate（公共 API）
- `rust-genai-types`：共享类型定义
- `rust-genai-macros`：过程宏

## 许可证

Apache-2.0。详见 `LICENSE`。

---

<a id="installation-1"></a>

**Language: [中文](#rust-gemini-sdk-rust-genai) | English**

> Community-maintained Rust SDK for the Google Gemini API and Vertex AI.
> Google maintains the official Go, JavaScript, and Python SDKs; this repository provides the community-maintained Rust implementation.

Rust SDK for the Google Gemini API and Vertex AI. This workspace aims to stay feature-aligned with the official Go, JavaScript, and Python SDKs while taking advantage of Rust's safety and performance.

## Features

- Async-first client with Tokio
- Gemini API and Vertex AI backends
- Models, Chats, Files, Caches, Batches, Operations
- Default automatic retries for `408`, `429`, `500`, `502`, `503`, and `504`
- Streaming (SSE)
- Structured JSON generation helper (`generate_json`)
- Event-level streaming helper (`generate_content_event_stream`)
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
rust-genai = "0.3.1"
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
        .generate_content("gemini-2.5-flash-lite", vec![Content::text("Hello from Rust!")])
        .await?;
    println!("{response:?}");
    Ok(())
}
```

## Documentation

- Getting Started: `docs/getting-started.md`
- Auth Guide: `docs/auth.md`
- Compatibility Matrix: `docs/compatibility-matrix.md`
- API Versions: `docs/api-versions.md`
- Error Handling: `docs/error-handling.md`
- Retry / Timeout: `docs/retry-timeout.md`
- Versioning and Stability: `docs/versioning.md`
- LLM Codegen Notes: `llms.txt`
- Official Sources: `docs/official-sources.md`
- Spec Sync: `docs/spec-sync.md`
- Best Practices: `docs/best-practices.md`
- MCP: `docs/mcp.md`
- Release Flow: `docs/release.md`
- API Reference: `cargo doc --open`

## Examples

See `rust-genai/examples/` for core features, error handling, performance, and Live Music.

## Stability Tiers

| Surface | Status | Scope |
|---------|--------|-------|
| Client, Models, Chats, Files, Caches, Batches, Operations, Tokens, Embeddings, SSE streaming | Stable | Core paths follow semantic-versioned evolution |
| Interactions API | Beta | Surface tracks official API changes quickly |
| Deep Research | Preview | Suitable for early workflow validation |
| Live Music | Experimental | Evaluate separately before wider adoption |
| MCP (`feature = "mcp"`) | Experimental | Feature-gated integration surface |

See `docs/versioning.md` for the release contract.

## Workspace Layout

- `rust-genai`: main client crate (public API)
- `rust-genai-types`: shared type definitions
- `rust-genai-macros`: procedural macros

## License

Apache-2.0. See `LICENSE`.
