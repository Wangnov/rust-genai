# Conformance Matrix Automation

This repo binds compatibility-matrix claims to executable tests under
`rust-genai/tests/conformance/`.

## Marker Semantics

- `mock_*`: no real credentials required. These tests are mock-backed and run
  in regular CI.
- `live_gemini_*`: requires `GEMINI_API_KEY` or `GOOGLE_API_KEY`. These tests
  run manually or in nightly automation.
- `live_vertex_*`: requires `GOOGLE_CLOUD_PROJECT`,
  `GOOGLE_CLOUD_LOCATION`, and ADC credentials via
  `GOOGLE_APPLICATION_CREDENTIALS`. These tests run manually or in nightly
  automation.
- `preview_*`: reserved for preview-only probes. These tests are informational
  and do not block releases.
- `expensive_*`: opt-in probes for higher-cost flows such as live file upload.
  These tests stay out of default CI and nightly runs.

## Test Layout

- `gemini_models.rs`: Gemini text generation, embeddings, token counting, and
  chat core paths.
- `vertex_models.rs`: Vertex text generation, embeddings, token counting, and
  compute tokens.
- `gemini_streaming.rs`: Gemini streaming and event-stream contracts.
- `vertex_streaming.rs`: Vertex streaming contracts.
- `gemini_files.rs`: Gemini files, file search stores, and documents.
- `vertex_guards.rs`: Vertex backend guards for Gemini-only surfaces.
- `retry_policy.rs`: default retries, overrides, retryability metadata, and
  attempt accounting.
- `json_generation.rs`: `generate_json` contract and MIME validation.

## Commands

Regular CI mock suite:

```bash
cargo test -p rust-genai --test conformance
```

Manual live Gemini suite:

```bash
GEMINI_API_KEY=... cargo test -p rust-genai --test conformance live_gemini_ -- --ignored --nocapture
```

Manual live Vertex suite:

```bash
GOOGLE_CLOUD_PROJECT=... \
GOOGLE_CLOUD_LOCATION=... \
GOOGLE_APPLICATION_CREDENTIALS=/path/to/adc.json \
  cargo test -p rust-genai --test conformance live_vertex_ -- --ignored --nocapture
```

Manual expensive suite:

```bash
GENAI_CONFORMANCE_ENABLE_EXPENSIVE=1 \
  GEMINI_API_KEY=... \
  cargo test -p rust-genai --test conformance expensive_ -- --ignored --nocapture
```

## CI Wiring

- `.github/workflows/ci.yml` runs the mock suite as part of the standard
  workspace test job.
- `.github/workflows/conformance-nightly.yml` runs the mock suite plus any live
  suites whose secrets are configured.
