# Release Guide

This repo publishes crates with a two-stage flow:

1. prepare the release commit with `cargo release`
2. run the release candidate workflow on `main`
3. push the final `vX.Y.Z` tag after the candidate passes

The detailed operator playbook lives in
[`docs/release.md`](./docs/release.md).

## Quick Commands

Patch release preparation:

```bash
cargo release patch --workspace --no-publish --no-tag --no-push --no-confirm --execute
git push origin main
```

Final release tag after the candidate workflow passes:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```
