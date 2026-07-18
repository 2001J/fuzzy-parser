# Fuzzy Parser

A Rust workspace for building a parsing engine with a CLI and future bindings.

## Local checks

Run the following commands before opening a pull request:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

To automatically format the codebase:

```bash
cargo fmt
```
