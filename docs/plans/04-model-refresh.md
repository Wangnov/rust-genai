# Plan 04: Model Refresh

## Goal

Refresh example and test model defaults so the repo points at current Gemini
model families and avoids short-horizon deprecation risk.

## Scope

- Move public text examples toward `gemini-3-flash-preview`.
- Move Live examples toward `gemini-3.1-flash-live-preview`.
- Refresh tests and mock fixtures away from `gemini-2.0-flash`.
- Update model capability helpers where current naming patterns changed.
- Keep specialized examples on the model family they require.

## Files

- `rust-genai/examples/*.rs`
- `rust-genai/tests/*.rs`
- `rust-genai/src/live.rs`
- `rust-genai/src/model_capabilities.rs`
- `rust-genai/src/tokenizer.rs`
- `README.md`
- `docs/*`

## Checks

- `cargo test -q`
- Spot-run key examples for text and Live flows where possible

## Status

- [x] Plan recorded
- [x] Public examples refreshed
- [x] Tests refreshed
- [x] Capability helpers refreshed
- [x] Verification complete
