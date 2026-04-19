# Retry and Timeout

`rust-genai` exposes timeout, proxy, and retry controls through `ClientBuilder`
and per-request `HttpOptions`. Retries run when `retry_options` is configured
on the client or on the request.

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
                initial_delay: Some(0.25),
                max_delay: Some(1.0),
                exp_base: Some(2.0),
                jitter: Some(0.1),
                http_status_codes: Some(vec![500]),
            }),
            ..Default::default()
        }),
        ..Default::default()
    })
    .await?;
```

## Disable Retries

Use `attempts: Some(1)` when you want a single request attempt with no retry
loop.

```rust
use rust_genai::types::http::HttpRetryOptions;

let client = rust_genai::Client::builder()
    .api_key("YOUR_API_KEY")
    .retry_options(HttpRetryOptions {
        attempts: Some(1),
        ..Default::default()
    })
    .build()?;
```

## Retry-After

When the backend sends `Retry-After`, `rust-genai` parses both `delay-seconds`
and HTTP-date values. The selected delay still respects `max_delay` when that
cap is configured. When the header is absent or unusable, the retry loop falls
back to exponential backoff.
