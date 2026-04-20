# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Conformance: add a dedicated `tests/conformance/` suite with mock, live Gemini, live Vertex, preview, and expensive markers plus a nightly/manual conformance workflow.
- Release engineering: add `SECURITY.md`, `RELEASE.md`, docs.rs verification, and semver checks for the workspace crates.
- Models: add `generate_json_with_schema` / `generate_json_with_schema_with_config` behind the `schemars` feature for schema-backed structured output.
- Streaming: aggregate final event-stream responses so `GenerateContentStreamEvent::Done` carries the combined text/tool result across stream chunks.
- Diagnostics: add optional `tracing` hooks for backend, model, attempt, status, retryability, retry-after, and latency fields.

### Changed
- Compatibility: bind the compatibility matrix to the new conformance test tiers and make mock vs live verification status explicit.
- Examples: update `generate_content_events` to print the aggregated final response from the `Done` event.
- Docs: document optional `schemars` and `tracing` features in the README installation and feature sections.

## [0.3.1] - 2026-04-20

### Added
- Models: add `generate_json` structured-output helper and event-stream examples for stream event consumption.
- Docs: add dedicated auth, compatibility matrix, error handling, retry/timeout, and versioning guides plus `llms.txt`.
- Validation: add a live Gemini smoke probe example and focused retry/stream/json coverage for the new SDK contracts.

### Changed
- Client: align `Client::from_env()` with official Vertex environment variables (`GOOGLE_GENAI_USE_VERTEXAI`, `GOOGLE_CLOUD_PROJECT`, `GOOGLE_CLOUD_LOCATION`, `GOOGLE_GENAI_API_VERSION`).
- Client: enable SDK default automatic retries for `408`, `429`, `500`, `502`, `503`, and `504` with bounded delay handling.
- Models: route streaming requests through backend-specific Gemini API and Vertex AI converters for schema parity with non-streaming calls.
- Release: verify publishability with `cargo package --workspace` in release candidate and release workflows.

### Fixed
- Errors: preserve structured API error metadata, parse `Retry-After` delay values, and bound retained headers/body metadata for safer diagnostics.
- Models: join segmented text parts for JSON parsing, enforce `application/json` for `generate_json`, and emit stream completion only after an explicit SSE done event.
- Backend selection: keep Gemini API precedence when an API key is configured and keep explicit Gemini/Vertex environment overrides deterministic.

## [0.3.0] - 2026-04-19

### Added
- Client: support configurable HTTP retries (global + per-request via `HttpOptions.retry_options`).
- Client: add a global `x-goog-api-client` SDK header and webhook client surface.
- Files: support `files:register` (register GCS URIs) for Gemini Developer API with OAuth/ADC.
- Models: `model_armor_config` support in `GenerateContentConfig` (Vertex AI only).
- Tunings: `encryption_spec` support in tuning job creation config (Vertex AI only).
- Tunings: add distillation method + Vertex OSS tuning fields (`tuning_mode`, `custom_base_model`, `output_uri`, teacher model options).
- Batches: `metadata` field in inlined responses.
- Webhooks: add webhook request types, verification helpers, and client methods for Gemini API webhook flows.
- Tokenizer: add `gemini-3-pro-preview` local tokenizer mapping.
- Interactions: support `include_input` (and related query params) on Get Interaction.
- Interactions: align create/get streaming with latest SSE event schema (event_id, content delta, error events).
- Interactions: add agent-based create (`agent`, `agent_config`) and interactions-specific tool + generation config types.
- Interactions: add `get_stream` / `get_stream_with_config`.
- Spec sync: add OpenAPI snapshot generation, manifest output, and a scheduled sync workflow.

### Changed
- DeepResearch: now uses the Deep Research agent (`deep-research-pro-preview-12-2025`) with `agent` / `agent_config` (instead of `model`).
- Docs: refresh installation snippets, official source references, API version guidance, and model examples across the docs set.
- Examples and tests: refresh default Gemini models and coverage for the latest SDK-facing surfaces.

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
