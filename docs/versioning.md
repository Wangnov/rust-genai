# Versioning and Stability

`rust-genai` uses semver with explicit stability tiers so application teams can
choose the right surface for each workload.

## Stable

Stable surfaces evolve with semantic versioning. Additive changes land in minor
releases. Breaking changes land in major releases.

- Client configuration and transport
- Models, Chats, Files, Caches, Batches, Operations
- Tokens and embeddings
- SSE streaming for `generate_content_stream`

## Beta

Beta surfaces track official APIs that are still moving quickly. Minor releases
may refine request or response shapes as upstream guidance evolves.

- Interactions API

## Preview

Preview surfaces follow Google preview APIs closely. Minor releases may add
fields, rename options, or refresh example flows to stay aligned.

- Deep Research

## Experimental

Experimental surfaces highlight fast iteration and feature-gated integrations.
Minor releases may revise these APIs to keep pace with upstream changes and SDK
ergonomics improvements.

- Live Music
- MCP (`feature = "mcp"`)

## Release Guidance

- Pick stable surfaces for long-lived production paths.
- Pick beta and preview surfaces for workloads that benefit from the newest API
  capabilities.
- Pick experimental surfaces for targeted integrations with dedicated rollout
  checks.
