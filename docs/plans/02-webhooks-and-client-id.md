# Plan 02: Webhooks And Client Identification

## Goal

Bring Rust parity closer to the current official Python and JavaScript SDKs by
adding webhook support and a global `x-goog-api-client` header strategy.

## Scope

- Add `WebhookConfig` support to batch and video generation config types.
- Add a `webhooks` service to the client surface.
- Implement `create`, `update`, `list`, `delete`, `get`, `ping`, and
  `rotate_signing_secret`.
- Add webhook resource types and tests.
- Add a default `x-goog-api-client` header for direct Gemini API requests.
- Preserve existing MCP usage suffix behavior.

## Files

- `rust-genai/src/client.rs`
- `rust-genai/src/webhooks.rs`
- `rust-genai/src/batches.rs`
- `rust-genai/src/models.rs`
- `rust-genai/src/models/builders.rs`
- `rust-genai-types/src/batches.rs`
- `rust-genai-types/src/models.rs`
- `rust-genai-types/src/webhooks.rs`
- `rust-genai-types/src/lib.rs`
- tests under `rust-genai/tests/`

## Checks

- Focused unit tests for webhook URLs and request bodies
- `cargo test -q`

## Status

- [x] Plan recorded
- [x] Type layer updated
- [x] Client and service layer updated
- [x] Header strategy updated
- [x] Verification complete
