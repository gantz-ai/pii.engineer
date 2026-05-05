# Contributing to PII Engineer

Thanks for your interest in contributing! Here's how to get started.

## Development Setup

```bash
git clone https://github.com/gantz-ai/pii.engineer.git
cd pii.engineer
cargo build --workspace
cargo test --workspace
```

Models auto-download from HuggingFace on first run. To start the server:

```bash
cargo run --release -p pii-engineer-server
# API at http://localhost:8000
```

## Making Changes

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Run `cargo test --workspace` and `cargo clippy --workspace`
4. Open a pull request

## What We're Looking For

- **New PII types** — add detection for additional entity types (tax IDs, medical record numbers, etc.)
- **Language support** — improve accuracy for underrepresented languages
- **Post-processing rules** — validation patterns for country-specific ID formats
- **Performance** — inference optimizations, batching, memory usage
- **Documentation** — examples, tutorials, API client libraries
- **Bug fixes** — especially false positives/negatives on real-world text

## Code Style

- Run `cargo clippy` before submitting
- Keep PRs focused — one feature or fix per PR
- Add tests for new functionality
- No comments unless the "why" is non-obvious

## Reporting Issues

- Include sample text that reproduces the issue (redact real PII)
- Specify the language of the input text
- Include the API response (entities, scores)

## License

By contributing, you agree that your contributions will be licensed under the [AGPL-3.0](LICENSE).
