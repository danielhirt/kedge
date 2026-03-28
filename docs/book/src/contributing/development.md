# Development

## Build

```bash
cargo build              # debug build
cargo build --release    # optimized release build
```

kedge compiles to a single static binary with no runtime dependencies beyond `git` on PATH. It uses rustls for TLS (no OpenSSL).

## Test

```bash
cargo test                                         # all tests (~45 tests)
cargo test --test fingerprint_test                 # single test file
cargo test --test detection_test -- detect_drift   # single test function
```

### Test structure

All tests live in `tests/` -- there are no `#[test]` blocks in `src/`. Test files:

| File | What it tests |
|------|--------------|
| `fingerprint_test.rs` | AST fingerprinting for all supported languages, whitespace/comment immunity, symbol scoping |
| `detection_test.rs` | Drift detection logic, sig-based and SHA-based provenance comparison |
| `frontmatter_test.rs` | YAML frontmatter parsing, provenance updates, batch updates |
| `config_test.rs` | `kedge.toml` parsing, defaults, missing fields |
| `models_test.rs` | Serialization/deserialization of data types |
| `triage_test.rs` | Prompt building, response parsing, classification application |
| `remediation_test.rs` | Payload construction, partition logic, auto-merge decisions |
| `agent_test.rs` | Agent process invocation, timeout handling |
| `output_test.rs` | Agent output parsing, URL scraping |
| `safety_test.rs` | Input validation: provenance, paths, URLs, git refs |
| `install_test.rs` | Steering file installation (copy and symlink modes) |
| `integration_test.rs` | End-to-end flows |
| `java_project_test.rs` | Full Java project fixture with realistic steering files |

### Test fixtures

Fixtures in `tests/fixtures/`:

- **Source files:** `AuthService.java`, `handler.go`, `component.tsx`, `auth.py`, `handler.rs`, `form.xml` -- sample code for fingerprint tests
- **Config:** `kedge.toml` -- sample configuration
- **Steering files:** `steering_with_anchors.md`, `steering_no_kedge.md` -- sample docs with and without kedge frontmatter
- **`java-project/`** -- a complete Java project with git history for integration tests
- **`java-project-docs/`** -- a matching docs repo with steering files

### Test conventions

- Use `tempfile::TempDir` for filesystem tests (auto-cleanup)
- Create real git repos with `git init` for tests that need git operations
- Tests are fast (~100ms total) -- no network calls, no AI inference

## Lint

```bash
cargo clippy
```

## Format

```bash
cargo fmt
```

## Run locally

```bash
cargo run -- init
cargo run -- check
cargo run -- status
cargo run -- link docs/my-file.md
cargo run -- install --link --group payments
```

Use `--config` to point to a non-default config file:

```bash
cargo run -- --config test-kedge.toml check
```
