# Compatibility Matrix

This matrix tracks current backend support and the verification depth that
exists in this repo.

## Legend

- `✅ smoke`: supported and covered by the mock-backed smoke suite in
  `rust-genai/tests/api_smoke_*`.
- `🧪 implemented`: public API exists and has unit or integration coverage in
  this repo.
- `⚠️ preview`: preview or experimental surface with repo coverage.
- `❌ guarded`: the SDK rejects this backend before sending the request.
- `SDK extension`: feature implemented by `rust-genai` itself rather than a
  first-party Google API family.

| API Surface | Gemini Developer API | Vertex AI | Verification |
|-------------|----------------------|-----------|--------------|
| Client + auth | ✅ smoke | ✅ smoke | builder tests + backend smoke suites |
| Models: `generateContent` | ✅ smoke | ✅ smoke | request conversion tests + smoke suites |
| Models: `streamGenerateContent` | ✅ smoke | ✅ smoke | SSE/converter tests + smoke suites |
| Models: embeddings | ✅ smoke | ✅ smoke | smoke suites |
| Models: count tokens | ✅ smoke | ✅ smoke | smoke suites |
| Models: compute tokens | ❌ guarded | ✅ smoke | backend guard + Vertex smoke |
| Chats | 🧪 implemented | 🧪 implemented | chat integration tests over models |
| Files | ✅ smoke | ❌ guarded | Gemini smoke + `files.rs` backend guard |
| File Search Stores | ✅ smoke | ❌ guarded | Gemini smoke + `file_search_stores.rs` guard |
| Documents | ✅ smoke | ❌ guarded | Gemini smoke + `documents.rs` guard |
| Caches | ✅ smoke | ✅ smoke | smoke suites |
| Batches | ✅ smoke | ✅ smoke | smoke suites |
| Operations | ✅ smoke | ✅ smoke | smoke suites |
| Tunings | ✅ smoke | ✅ smoke | smoke suites |
| Live API | 🧪 implemented | 🧪 implemented | websocket integration tests |
| Live Music | ⚠️ experimental | ❌ guarded | websocket tests + `live_music.rs` guard |
| Interactions API | ⚠️ preview | ❌ guarded | Gemini smoke + `interactions.rs` guard |
| Deep Research | ⚠️ preview | ❌ guarded | wrapper tests + Gemini smoke through Interactions |
| MCP (`feature = "mcp"`) | SDK extension | SDK extension | feature-gated examples and manual checks |

## Notes

- The smoke suites in `rust-genai/tests/api_smoke_*.rs` are mock-backed. They
  validate request shaping, routing, and response parsing inside the SDK.
- `❌ guarded` means the SDK currently returns `InvalidConfig` for that
  backend.
- Manual live validation for this PR covers the core Gemini generation paths.
  Promote a surface beyond `🧪 implemented` or `⚠️ preview` after backend-
  specific live smoke coverage exists.
- Stability guarantees for each surface live in `docs/versioning.md`.
