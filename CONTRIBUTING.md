# Contributing

Thanks for your interest in contributing to the Rust Gemini SDK.

## How to contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m "Add amazing feature"`)
4. Push to your fork (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Development guidelines

- Run `cargo fmt` before submitting changes
- Run `cargo clippy -D warnings` to keep the codebase clean
- Add tests for new functionality
- Update `CHANGELOG.md` when behavior changes

## Pre-commit hooks (recommended)

This repo ships a hook script in `rust-genai/.githooks`.

```bash
git config core.hooksPath rust-genai/.githooks
```
