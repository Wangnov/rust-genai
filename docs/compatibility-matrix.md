# Compatibility Matrix

This matrix tracks current backend support and the verification depth that
exists in this repo.

## Legend

- `✅ smoke`: supported and covered by the mock-backed smoke suite in
  `rust-genai/tests/conformance/*.rs` or the linked integration suite.
- `🧪 implemented`: public API exists and has unit or integration coverage in
  this repo.
- `⚠️ preview`: preview or experimental surface with repo coverage.
- `❌ guarded`: the SDK rejects this backend before sending the request.
- `SDK extension`: feature implemented by `rust-genai` itself rather than a
  first-party Google API family.

Marker semantics and manual commands live in
[`docs/conformance.md`](./conformance.md).

| API Surface | Gemini Developer API | Vertex AI | Verification |
|-------------|----------------------|-----------|--------------|
| Client + auth | ✅ smoke | ✅ smoke | builder tests + `tests/conformance/gemini_models.rs` + `tests/conformance/vertex_models.rs` |
| Models: `generateContent` | ✅ smoke | ✅ smoke | request conversion tests + `tests/conformance/gemini_models.rs` + `tests/conformance/vertex_models.rs` |
| Models: `streamGenerateContent` | ✅ smoke | ✅ smoke | SSE/converter tests + `tests/conformance/gemini_streaming.rs` + `tests/conformance/vertex_streaming.rs` |
| Models: embeddings | ✅ smoke | ✅ smoke | `tests/conformance/gemini_models.rs` + `tests/conformance/vertex_models.rs` |
| Models: count tokens | ✅ smoke | ✅ smoke | `tests/conformance/gemini_models.rs` + `tests/conformance/vertex_models.rs` |
| Models: compute tokens | ❌ guarded | ✅ smoke | backend guard + `tests/conformance/vertex_models.rs` |
| Chats | 🧪 implemented | 🧪 implemented | `tests/conformance/gemini_models.rs` + `tests/chats_api.rs` |
| Files | ✅ smoke | ❌ guarded | `tests/conformance/gemini_files.rs` + `tests/conformance/vertex_guards.rs` |
| File Search Stores | ✅ smoke | ❌ guarded | `tests/conformance/gemini_files.rs` + `tests/conformance/vertex_guards.rs` |
| Documents | ✅ smoke | ❌ guarded | `tests/conformance/gemini_files.rs` + `tests/conformance/vertex_guards.rs` |
| Caches | ✅ smoke | ✅ smoke | `tests/caches_api.rs` |
| Batches | ✅ smoke | ✅ smoke | `tests/batches_api.rs` |
| Operations | ✅ smoke | ✅ smoke | `tests/operations_api.rs` |
| Tunings | ✅ smoke | ✅ smoke | `tests/tunings_api.rs` |
| Live API | 🧪 implemented | 🧪 implemented | `tests/live_ws.rs` |
| Live Music | ⚠️ experimental | ❌ guarded | websocket tests + `live_music.rs` guard |
| Interactions API | ⚠️ preview | ❌ guarded | `tests/interactions_api.rs` + backend guards |
| Deep Research | ⚠️ preview | ❌ guarded | wrapper tests + Gemini interaction coverage |
| MCP (`feature = "mcp"`) | SDK extension | SDK extension | feature-gated examples and manual checks |

## Notes

- The `mock_*` tests in `rust-genai/tests/conformance/*.rs` are mock-backed.
  They validate request shaping, routing, and response parsing inside the SDK.
- `❌ guarded` means the SDK currently returns `InvalidConfig` for that
  backend.
- The live conformance markers cover core Gemini and Vertex generation paths.
  Promote a surface beyond `🧪 implemented` or `⚠️ preview` after backend-
  specific live automation exists for that surface.
- Stability guarantees for each surface live in `docs/versioning.md`.
