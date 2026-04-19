# Error Handling

All SDK methods return `Result<T, rust_genai::Error>`.

## Structured API Errors

`Error::ApiError` now carries parsed metadata from Google-style error payloads.

```rust
match client.models().list().await {
    Ok(models) => println!("models: {}", models.models.len()),
    Err(err) => {
        if err.is_rate_limited() {
            eprintln!("retry after: {:?}", err.retry_after());
        }
        eprintln!("status: {:?}", err.status());
        eprintln!("code: {:?}", err.code());
        eprintln!("attempts: {:?}", err.attempts());
        eprintln!("body: {:?}", err.body());
    }
}
```

## Useful Helpers

- `status()` returns the HTTP status code when the failure came from an API.
- `code()` returns the Google error code such as `RESOURCE_EXHAUSTED`.
- `details()` returns structured `error.details` content when the backend sends it.
- `retry_after()` returns the parsed `Retry-After` delay when present.
- `attempts()` returns the number of HTTP attempts recorded by the retry loop.
- `is_retryable()` and `is_rate_limited()` make policy decisions easy to express.
