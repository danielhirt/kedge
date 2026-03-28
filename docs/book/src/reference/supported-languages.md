# Supported Languages

kedge uses [tree-sitter](https://tree-sitter.github.io/tree-sitter/) for AST-based fingerprinting. Supported languages get whitespace/comment-immune, symbol-scoped fingerprints. All other file types fall back to SHA-256 content hashing.

## AST fingerprinting

| Language | Extensions | Symbol syntax | Example |
|----------|------------|---------------|---------|
| Java | `.java` | `ClassName#methodName` | `AuthService#validateToken` |
| Go | `.go` | `FunctionName` | `HandleRequest` |
| TypeScript | `.ts`, `.tsx`, `.js`, `.jsx` | `ClassName#methodName` or `functionName` | `UserService#create` |
| Python | `.py` | `ClassName#method_name` or `function_name` | `AuthService#validate_token` |
| Rust | `.rs` | `StructName#method_name` or `function_name` | `TokenStore#get` |
| XML | `.xml` | (file-level only) | -- |

### How AST fingerprinting works

1. kedge parses the source file using the language's tree-sitter grammar
2. If a `symbol` is specified, kedge navigates to that symbol's AST subtree
3. It walks the AST, hashing each node's kind and leaf token text
4. Comment nodes (`line_comment`, `block_comment`, `comment`, `multiline_comment`) are skipped
5. The hash is truncated to 16 hex characters: `sig:a1b2c3d4e5f67890`

### Properties

- **Whitespace immune** -- reformatting, indentation changes, and blank line additions don't change the fingerprint
- **Comment immune** -- adding, removing, or editing comments doesn't change the fingerprint
- **Rebase/squash safe** -- the fingerprint is computed from code structure, not git history
- **Symbol-scoped** -- when tracking a specific method, changes to other methods in the same file don't trigger drift

## Symbol resolution

### `ClassName#methodName` (Java, TypeScript, Python, Rust)

kedge finds the class/struct declaration matching `ClassName`, then searches its body for a method matching `methodName`.

- **Java**: matches `class_declaration` and `method_declaration` / `constructor_declaration` nodes
- **TypeScript/TSX**: matches `class_declaration` / `class` and `method_definition` / `function_declaration` nodes
- **Python**: matches `class_definition` and `function_definition` nodes; handles `decorated_definition` wrappers (decorators are included in the fingerprint)
- **Rust**: matches `impl_item` blocks by type name and `function_item` nodes within

### `functionName` (Go, TypeScript, Python, Rust)

kedge searches for a top-level function declaration matching the name. No class scoping.

- **Go**: matches `function_declaration` and `method_declaration` nodes
- **TypeScript**: matches `function_declaration` nodes
- **Python**: matches `function_definition` nodes
- **Rust**: matches `function_item` nodes

### XML

XML files are always fingerprinted at file level. Symbol scoping is not supported.

## Content-hash fallback

Files with extensions not matching any supported language (or when AST parsing fails) fall back to SHA-256 content hashing:

1. Hash the raw file content with SHA-256
2. Truncate to 16 hex characters: `sig:a1b2c3d4e5f67890`

The fallback hashes **raw content**, so whitespace and comment changes will register as drift. This is intentional -- without AST understanding, kedge can't distinguish structural changes from cosmetic ones.

## Adding language support

kedge's language support is extensible. See [Adding a Language](../contributing/adding-a-language.md) for a step-by-step guide to adding a new tree-sitter grammar.
