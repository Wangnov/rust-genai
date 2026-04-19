# Compatibility Matrix

This matrix shows the current support posture across Gemini Developer API,
Vertex AI, tests, and examples.

| API Surface | Gemini Developer API | Vertex AI | Unit/Mock Tests | Live Smoke | Example |
|-------------|----------------------|-----------|-----------------|------------|---------|
| Client + auth | ✅ | ✅ | ✅ | ✅ | ✅ |
| Models: `generateContent` | ✅ | ✅ | ✅ | ✅ | ✅ |
| Models: `streamGenerateContent` | ✅ | ✅ | ✅ | ✅ | ✅ |
| Models: embeddings | ✅ | ✅ | ✅ | ✅ | ✅ |
| Models: count tokens | ✅ | ✅ | ✅ | ✅ | ✅ |
| Models: compute tokens | - | ✅ | ✅ | ✅ | ✅ |
| Chats | ✅ | ✅ | ✅ | ✅ | ✅ |
| Files | ✅ | ✅ | ✅ | ✅ | ✅ |
| File Search Stores / Documents | ✅ | ✅ | ✅ | ✅ | ✅ |
| Caches | ✅ | ✅ | ✅ | ✅ | ✅ |
| Batches | ✅ | ✅ | ✅ | ✅ | ✅ |
| Operations | ✅ | ✅ | ✅ | ✅ | ✅ |
| Tunings | - | ✅ | ✅ | ✅ | - |
| Live API | ✅ | ✅ | ✅ | ✅ | ✅ |
| Live Music | - | ✅ | ✅ | - | ✅ |
| Interactions API | ✅ | ✅ | ✅ | ✅ | ✅ |
| Deep Research | ✅ | ✅ | ✅ | ✅ | ✅ |
| MCP (`feature = "mcp"`) | SDK extension | SDK extension | ✅ | manual | ✅ |

## Notes

- `SDK extension` means the surface is implemented inside `rust-genai` and does
  not map to a first-party Google API family.
- `Live Smoke` reflects the current smoke-test suite in `rust-genai/tests/`.
- Example coverage lives in `rust-genai/examples/`.
- Stability guarantees for each surface live in `docs/versioning.md`.
