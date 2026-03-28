# Adding a Language

kedge uses tree-sitter for AST-based fingerprinting. Adding a new language involves six steps, all in `src/detection/fingerprint.rs` and `Cargo.toml`.

## Prerequisites

- A tree-sitter grammar crate for the language (search [crates.io](https://crates.io/search?q=tree-sitter) for `tree-sitter-<language>`)
- Familiarity with the language's AST node types (check the grammar's `node-types.json` or tree-sitter playground)

## Step 1: Add the `Language` enum variant

In `src/detection/fingerprint.rs`, add a variant to the `Language` enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Java,
    Go,
    TypeScript,
    Tsx,
    Xml,
    Python,
    Rust,
    Kotlin,  // new
}
```

## Step 2: Map file extensions

In `Language::from_extension()`, add the extension mapping:

```rust
pub fn from_extension(ext: &str) -> Option<Language> {
    match ext {
        // ... existing mappings ...
        "kt" | "kts" => Some(Language::Kotlin),
        _ => None,
    }
}
```

## Step 3: Add the grammar dependency

In `Cargo.toml`, add the tree-sitter grammar crate:

```toml
[dependencies]
tree-sitter-kotlin = "0.3"  # check crates.io for the latest version
```

## Step 4: Implement `tree_sitter_language()`

In the `tree_sitter_language()` method, add the match arm:

```rust
fn tree_sitter_language(&self) -> tree_sitter::Language {
    match self {
        // ... existing arms ...
        Language::Kotlin => tree_sitter_kotlin::LANGUAGE.into(),
    }
}
```

> **Note:** Some grammar crates export `LANGUAGE` as a constant, others use a function like `language()`. Check the crate's documentation.

## Step 5: Add comment node kinds

Check the `is_comment()` function. The current implementation handles the most common node kinds:

```rust
fn is_comment(kind: &str, _lang: Language) -> bool {
    matches!(
        kind,
        "line_comment" | "block_comment" | "comment" | "multiline_comment"
    )
}
```

If your language uses different comment node kinds, add them. Check the grammar's `node-types.json` or use the tree-sitter playground to parse a file with comments and inspect the AST.

## Step 6: Add symbol resolution (if applicable)

If the language supports class/method scoping, update the `find_symbol_recursive()` function. You'll need to:

1. Identify the AST node kinds for class declarations (e.g., `class_declaration`)
2. Identify the AST node kinds for method/function declarations (e.g., `function_declaration`)
3. Add a match arm in `is_class_declaration()` and `is_method_declaration()`
4. Add a match arm in `find_symbol_recursive()` for navigating class bodies

For example, Kotlin class/method resolution might look like:

```rust
fn is_method_declaration(kind: &str, lang: Language) -> bool {
    match lang {
        // ... existing arms ...
        Language::Kotlin => matches!(kind, "function_declaration"),
        // ...
    }
}
```

If the language only supports file-level fingerprinting (like XML), add a no-op arm that returns `None` in `find_symbol_recursive()`.

## Step 7: Add tests

Create test fixtures in `tests/fixtures/` and add fingerprint tests in `tests/fingerprint_test.rs`:

```rust
#[test]
fn fingerprint_kotlin_method() {
    let source = std::fs::read_to_string("tests/fixtures/service.kt").unwrap();
    let sig = compute_sig(&source, "service.kt", Some("UserService#create"));
    assert!(sig.starts_with("sig:"));
    assert_eq!(sig.len(), 4 + 16); // "sig:" + 16 hex chars
}

#[test]
fn fingerprint_kotlin_whitespace_immune() {
    let source1 = "class Foo {\n  fun bar() { return 1 }\n}";
    let source2 = "class Foo {\n    fun bar() {\n        return 1\n    }\n}";
    let sig1 = compute_sig(source1, "foo.kt", Some("Foo#bar"));
    let sig2 = compute_sig(source2, "foo.kt", Some("Foo#bar"));
    assert_eq!(sig1, sig2);
}

#[test]
fn fingerprint_kotlin_comment_immune() {
    let source1 = "fun greet() { println(\"hi\") }";
    let source2 = "// A greeting function\nfun greet() { println(\"hi\") }";
    let sig1 = compute_sig(source1, "greet.kt", Some("greet"));
    let sig2 = compute_sig(source2, "greet.kt", Some("greet"));
    assert_eq!(sig1, sig2);
}
```

Run the tests:

```bash
cargo test --test fingerprint_test
```

## Checklist

- [ ] `Language` enum variant added
- [ ] File extension mapped in `from_extension()`
- [ ] Grammar dependency added to `Cargo.toml`
- [ ] `tree_sitter_language()` match arm added
- [ ] Comment node kinds verified in `is_comment()`
- [ ] Symbol resolution implemented (or file-level only)
- [ ] Tests added and passing
- [ ] `detection.languages` documented with the new language name
