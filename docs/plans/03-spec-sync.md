# Plan 03: Spec Sync

## Goal

Create a repeatable sync path from Google's official discovery/OpenAPI document
to this workspace so new fields, resources, and deprecations surface quickly.

## Scope

- Add a script that fetches the official Gemini discovery/OpenAPI document.
- Store a normalized snapshot in the repo for review.
- Add a diff workflow that highlights shape changes.
- Document how to run the sync locally and how to review drift.
- Keep the mechanism lightweight and readable.

## Files

- `scripts/spec_sync.sh`
- `scripts/spec_sync.py`
- `spec/openapi3_0.json`
- `spec/api-versions.md`
- `spec/deprecations.md`
- `spec/manifest.json`
- `docs/spec-sync.md`
- `.github/workflows/spec-sync.yml`

## Checks

- Run the script locally
- Confirm the workflow file validates as YAML

## Status

- [x] Plan recorded
- [x] Script added
- [x] Snapshot added
- [x] Workflow added
- [x] Verification complete
