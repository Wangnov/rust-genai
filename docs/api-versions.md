# API Versions

`rust-genai` defaults to the preview-capable Google endpoints so the SDK can
cover the newest Gemini and Vertex AI features.

## Defaults

- Gemini Developer API: `v1beta`
- Vertex AI: `v1beta1`

These defaults come from the client transport layer and match the current
official SDK posture for preview-feature coverage.

## When To Use Each Version

- Use `v1` when you want the most stable surface and your workflow only depends
  on GA features.
- Use `v1beta` for Gemini Developer API features that land ahead of `v1`.
- Use `v1beta1` for Vertex AI features that are still in the preview-oriented
  surface.
- Use `v1alpha` only for flows that explicitly require it, such as current
  ephemeral-token Live API paths.

## Gemini Developer API Example

```rust
use rust_genai::Client;

let client = Client::builder()
    .api_key("YOUR_API_KEY")
    .api_version("v1")
    .build()?;
```

## Vertex AI Example

```rust
use rust_genai::{Backend, Client};

let client = Client::builder()
    .backend(Backend::VertexAi)
    .vertex_project("my-project")
    .vertex_location("us-central1")
    .api_version("v1beta1")
    .build()?;
```

## Environment Variable Override

`Client::from_env()` respects `GENAI_API_VERSION` when it is set.

```bash
export GENAI_API_VERSION=v1
```

## Guidance

- Check the official [API versions guide](https://ai.google.dev/gemini-api/docs/api-versions)
  before moving a default.
- Check the official [deprecations page](https://ai.google.dev/gemini-api/docs/deprecations)
  before pinning a model into docs or examples.
