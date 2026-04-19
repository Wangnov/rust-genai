# Auth Guide

`rust-genai` supports the same two primary deployment paths as the Google Gen
AI SDKs: Gemini Developer API and Vertex AI.

## Gemini Developer API

Set one of these environment variables:

```bash
export GEMINI_API_KEY="YOUR_API_KEY"
# or
export GOOGLE_API_KEY="YOUR_API_KEY"
```

Then build the client with:

```rust
let client = rust_genai::Client::from_env()?;
```

## Vertex AI with Official Environment Variables

Set the official Vertex switch and project metadata:

```bash
export GOOGLE_GENAI_USE_VERTEXAI=true
export GOOGLE_CLOUD_PROJECT="my-project"
export GOOGLE_CLOUD_LOCATION="us-central1"
export GOOGLE_GENAI_API_VERSION="v1"
```

`Client::from_env()` will build a Vertex AI client and use Application Default
Credentials.

## OAuth and ADC

For explicit credential flows, use the dedicated constructors:

```rust
let oauth_client = rust_genai::Client::with_oauth("client_secret.json")?;
let adc_client = rust_genai::Client::with_adc()?;
```

## Base URL Overrides

Use these environment variables for local gateways, mocks, or proxies:

```bash
export GOOGLE_GENAI_BASE_URL="https://example.internal/"
# compatible fallbacks:
export GENAI_BASE_URL="https://example.internal/"
export GEMINI_BASE_URL="https://example.internal/"
```
