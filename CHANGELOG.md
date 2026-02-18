# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Client: support configurable HTTP retries (global + per-request via `HttpOptions.retry_options`).
- Files: support `files:register` (register GCS URIs) for Gemini Developer API with OAuth/ADC.
- Models: `model_armor_config` support in `GenerateContentConfig` (Vertex AI only).
- Tunings: `encryption_spec` support in tuning job creation config (Vertex AI only).
- Tunings: add distillation method + Vertex OSS tuning fields (`tuning_mode`, `custom_base_model`, `output_uri`, teacher model options).
- Batches: `metadata` field in inlined responses.
- Tokenizer: add `gemini-3-pro-preview` local tokenizer mapping.
- Interactions: support `include_input` (and related query params) on Get Interaction.
- Interactions: align create/get streaming with latest SSE event schema (event_id, content delta, error events).
- Interactions: add agent-based create (`agent`, `agent_config`) and interactions-specific tool + generation config types.
- Interactions: add `get_stream` / `get_stream_with_config`.

### Changed
- DeepResearch: now uses the Deep Research agent (`deep-research-pro-preview-12-2025`) with `agent` / `agent_config` (instead of `model`).

## [0.1.0] - 2025-12-26

### Changed
- Switched ADC implementation to official `google-cloud-auth`.
- MSRV bumped to Rust 1.88 for official Google Cloud Rust libraries.

### Added
- Full Gemini API + Vertex AI coverage for Models, Chats, Files, Caches, Batches, Tunings, Operations.
- Streaming SSE responses and unified error handling.
- Live API sessions (resume, compression) and Live Music (experimental).
- Tooling system including search, maps, code execution, URL context, computer use, and file search.
- Thinking controls, grounding metadata, logprobs, and media resolution settings.
- Media generation for images, videos, and audio.
- OAuth/ADC authentication and ephemeral token support.
- Extensive examples and docs for common workflows.
- Interactions API (Beta) client with SSE updates.
- Deep Research convenience wrapper and examples.
- Model capability gating for function response media and code execution with images.
- Native audio Live API and TTS multi-speaker examples.
- Optional local token estimation via Kitoken feature.
- ComputeTokens (Vertex AI) API and local compute_tokens via Kitoken.
- Experimental MCP support (feature: `mcp`) with rmcp integration and stdio example.
- Automatic MCP usage label injection in request headers when MCP tools are used.
- Automatic function calling (AFC) for callable tools with non-streaming and streaming support.
- FunctionResponse::from_mcp_response convenience helper (feature: `mcp`).
- Recontext Image and Segment Image (Vertex AI) models support.
- Retrieval tool types including Vertex RAG Store, External API, and Enterprise Web Search.
- New examples for recontext_image and segment_image.
- Tunings API (tune/get/list/cancel) support and tunings_basic example.
- Models update/delete management support.
- Convenience pagination helpers (`all`) and chat send/send_stream aliases.
