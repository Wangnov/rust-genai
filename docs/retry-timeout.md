# Retry and Timeout

`rust-genai` exposes timeout, proxy, and retry controls through `ClientBuilder`
and per-request `HttpOptions`.

## Global Client Settings

```rust
use rust_genai::types::http::HttpRetryOptions;

let client = rust_genai::Client::builder()
    .api_key("YOUR_API_KEY")
    .timeout(30)
    .retry_options(HttpRetryOptions {
        attempts: Some(3),
        initial_delay: Some(0.5),
        max_delay: Some(4.0),
        exp_base: Some(2.0),
        jitter: Some(0.25),
        http_status_codes: Some(vec![408, 429, 500, 502, 503, 504]),
    })
    .build()?;
```

## Per-request Overrides

```rust
use rust_genai::types::caches::ListCachedContentsConfig;
use rust_genai::types::http::{HttpOptions, HttpRetryOptions};

let response = client
    .caches()
    .list_with_config(ListCachedContentsConfig {
        http_options: Some(HttpOptions {
            retry_options: Some(HttpRetryOptions {
                attempts: Some(2),
                initial_delay: Some(0.0),
                max_delay: Some(0.0),
                exp_base: Some(0.0),
                jitter: Some(0.0),
                http_status_codes: Some(vec![500]),
            }),
            ..Default::default()
        }),
        ..Default::default()
    })
    .await?;
```

## Retry-After

When the backend sends a numeric `Retry-After` header, the retry loop honors
that delay before falling back to exponential backoff.
