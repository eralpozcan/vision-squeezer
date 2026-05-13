# Contributing

## Setup

```bash
git clone https://github.com/eralpozcan/vision-squeezer.git
cd vision-squeezer
cargo build
```

## Workflow

1. Fork → branch → PR against `main`
2. Keep PRs focused — one thing per PR
3. Run before submitting:

```bash
cargo fmt
cargo clippy
cargo test
```

## Adding a New Provider

Token estimation lives in `src/lib.rs` → `estimate_tokens()`. Add a variant to `VisionModel` and a match arm there.

## Adding a New Output Format

`OutputFormat` enum + match arm in `encode_to_bytes()` in `src/lib.rs`.

## Reporting Bugs

Use the [bug report template](https://github.com/eralpozcan/vision-squeezer/issues/new/choose).

## License

By contributing, you agree your code will be licensed under the [Elastic License 2.0](LICENSE).
