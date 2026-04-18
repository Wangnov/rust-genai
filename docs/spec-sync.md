# Spec Sync

This repo tracks a small set of official Gemini sources in `spec/` so upstream
surface changes stay reviewable in Git.

## Tracked Sources

- OpenAPI discovery snapshot: `spec/openapi3_0.json`
- API version guide mirror: `spec/api-versions.md`
- Deprecation guide mirror: `spec/deprecations.md`
- Snapshot manifest with hashes and source URLs: `spec/manifest.json`

## Local Run

```bash
bash scripts/spec_sync.sh
git diff -- spec
```

## Review Flow

1. Run the sync script.
2. Inspect `git diff -- spec` for new resources, fields, or model lifecycle changes.
3. Map drift into code and docs updates:
   - request/response types in `rust-genai-types`
   - client surfaces and request builders in `rust-genai`
   - examples, guides, and release notes
4. Refresh tests for any new request body or path behavior.

## CI Behavior

`.github/workflows/spec-sync.yml` runs on a weekly schedule and on manual
dispatch. When the tracked sources move, the workflow uploads the refreshed
`spec/` directory as an artifact and exits with a failing status so the drift is
visible immediately.
