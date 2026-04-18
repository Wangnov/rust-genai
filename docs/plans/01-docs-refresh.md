# Plan 01: Docs Refresh

## Goal

Refresh public-facing documentation so the repo presents current crate versions,
current Google SDK conventions, and a clear path for API-version selection.

## Scope

- Update all public install snippets to `rust-genai = "0.2.0"`.
- Refresh top-level docs paths and remove hard-coded local machine paths.
- Add an API-version guide for `v1`, `v1beta`, `v1beta1`, and `v1alpha`.
- Add an official-source reference list with canonical URLs and `.md` mirrors.
- Align quickstart language with the current Google GenAI SDK family.

## Files

- `README.md`
- `docs/getting-started.md`
- `docs/best-practices.md`
- `docs/mcp.md`
- `rust-genai/examples/README.md`
- `docs/api-versions.md`
- `docs/official-sources.md`

## Checks

- `cargo test -q`
- Read every updated doc entry once for version and path consistency.

## Status

- [x] Plan recorded
- [x] Public docs updated
- [x] New reference docs added
- [x] Verification complete
