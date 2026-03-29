# Contributing

Thanks for your interest in kedge. This guide covers what you need to get started.

## Setup

Requires Rust 1.70+ and `git` on PATH.

```bash
git clone https://github.com/danielhirt/kedge.git
cd kedge
cargo build
cargo test
```

## Development loop

```bash
cargo build                          # Debug build
cargo test                           # All tests (225 across 17 suites)
cargo test --test fingerprint_test   # Single test file
cargo test -- test_name              # Single test function
cargo clippy                         # Lint
cargo fmt                            # Format
cargo run -- <command>               # Run CLI locally
```

All tests run in under 3 seconds. No external services or API keys required.

## Test structure

Tests live in `tests/` (no `#[test]` in `src/`). Each test file covers one module:

| File | Covers |
|------|--------|
| `fingerprint_test.rs` | AST fingerprinting across all languages |
| `detection_test.rs` | Drift detection with real git repos |
| `triage_test.rs` | Prompt building, response parsing, classification |
| `remediation_test.rs` | Agent payload construction, auto-merge logic |
| `agent_test.rs` | Subprocess spawning, timeout, env var expansion |
| `config_test.rs` | TOML parsing, defaults, backwards compatibility |
| `frontmatter_test.rs` | YAML frontmatter extraction, provenance updates |
| `integration_test.rs` | End-to-end CLI tests via `assert_cmd` |
| `install_test.rs` | Steering file distribution (symlink/copy modes) |
| `repo_cache_test.rs` | Git clone/fetch caching |

Tests that need git repos create them in `tempfile::TempDir`. Fixtures live in `tests/fixtures/`.

## Adding a new language

1. Add a variant to the `Language` enum in `src/detection/fingerprint.rs`
2. Map the file extension in `Language::from_extension()`
3. Add the tree-sitter grammar dependency to `Cargo.toml`
4. Implement the `tree_sitter_language()` match arm
5. Add `is_comment()` node kinds if they differ from the defaults
6. Add fingerprint tests in `tests/fingerprint_test.rs`

## Pull requests

- Run `cargo test`, `cargo clippy`, and `cargo fmt --check` before submitting
- Each PR should include tests that prove the change works
- Keep PRs focused on a single change

## Reporting issues

File issues at [github.com/danielhirt/kedge/issues](https://github.com/danielhirt/kedge/issues). Include the kedge version (`kedge --version`), your OS, and steps to reproduce.

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 License.
