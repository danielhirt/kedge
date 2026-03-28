use kedge::detection::fingerprint::{
    ast_fingerprint, compute_sig, content_hash, Language, SIG_PREFIX,
};

#[test]
fn content_hash_produces_consistent_output() {
    let content = "public class Foo { }";
    let hash1 = content_hash(content);
    let hash2 = content_hash(content);
    assert_eq!(hash1, hash2);
    assert!(!hash1.is_empty());
}

#[test]
fn content_hash_differs_for_different_content() {
    let hash1 = content_hash("public class Foo { }");
    let hash2 = content_hash("public class Bar { }");
    assert_ne!(hash1, hash2);
}

#[test]
fn ast_fingerprint_ignores_whitespace_changes() {
    let compact = "public class Foo { public void bar() { return; } }";
    let spaced = "public class Foo {\n    public void bar() {\n        return;\n    }\n}";
    let hash1 = ast_fingerprint(compact, Language::Java, None).unwrap();
    let hash2 = ast_fingerprint(spaced, Language::Java, None).unwrap();
    assert_eq!(hash1, hash2);
}

#[test]
fn ast_fingerprint_detects_code_changes() {
    let before = "public class Foo { public void bar() { return; } }";
    let after = "public class Foo { public void bar(int x) { return; } }";
    let hash1 = ast_fingerprint(before, Language::Java, None).unwrap();
    let hash2 = ast_fingerprint(after, Language::Java, None).unwrap();
    assert_ne!(hash1, hash2);
}

#[test]
fn ast_fingerprint_with_symbol_filters_to_method() {
    let source = std::fs::read_to_string("tests/fixtures/AuthService.java").unwrap();
    let full_hash = ast_fingerprint(&source, Language::Java, None).unwrap();
    let method_hash =
        ast_fingerprint(&source, Language::Java, Some("AuthService#validateToken")).unwrap();
    // Symbol-scoped hash should differ from full-file hash
    assert_ne!(full_hash, method_hash);
    // Symbol hash should be consistent
    let method_hash2 =
        ast_fingerprint(&source, Language::Java, Some("AuthService#validateToken")).unwrap();
    assert_eq!(method_hash, method_hash2);
}

#[test]
fn ast_fingerprint_ignores_comment_changes() {
    let before = "public class Foo { public void bar() { return; } }";
    let after = "public class Foo { /* added comment */ public void bar() { return; } }";
    let hash1 = ast_fingerprint(before, Language::Java, None).unwrap();
    let hash2 = ast_fingerprint(after, Language::Java, None).unwrap();
    assert_eq!(hash1, hash2);
}

#[test]
fn language_detection_from_extension() {
    assert_eq!(Language::from_extension("java"), Some(Language::Java));
    assert_eq!(Language::from_extension("go"), Some(Language::Go));
    assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
    assert_eq!(Language::from_extension("js"), Some(Language::TypeScript));
    assert_eq!(Language::from_extension("tsx"), Some(Language::Tsx));
    assert_eq!(Language::from_extension("jsx"), Some(Language::Tsx));
    assert_eq!(Language::from_extension("xml"), Some(Language::Xml));
    assert_eq!(Language::from_extension("py"), Some(Language::Python));
    assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
    assert_eq!(Language::from_extension("txt"), None);
}

#[test]
fn go_fingerprint_ignores_whitespace() {
    let compact = "package main\nfunc foo() { return }";
    let spaced = "package main\n\nfunc foo() {\n\treturn\n}";
    let h1 = ast_fingerprint(compact, Language::Go, None).unwrap();
    let h2 = ast_fingerprint(spaced, Language::Go, None).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn go_fingerprint_detects_changes() {
    let source = std::fs::read_to_string("tests/fixtures/handler.go").unwrap();
    let h1 = ast_fingerprint(&source, Language::Go, None).unwrap();
    let modified = source.replace("http.StatusUnauthorized", "http.StatusForbidden");
    let h2 = ast_fingerprint(&modified, Language::Go, None).unwrap();
    assert_ne!(h1, h2);
}

#[test]
fn tsx_fingerprint_parses_jsx() {
    let source = std::fs::read_to_string("tests/fixtures/component.tsx").unwrap();
    let h1 = ast_fingerprint(&source, Language::Tsx, None).unwrap();
    assert!(!h1.is_empty());
    // Consistent
    let h2 = ast_fingerprint(&source, Language::Tsx, None).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn tsx_fingerprint_detects_changes() {
    let source = std::fs::read_to_string("tests/fixtures/component.tsx").unwrap();
    let h1 = ast_fingerprint(&source, Language::Tsx, None).unwrap();
    let modified = source.replace("onLogout", "onSignOut");
    let h2 = ast_fingerprint(&modified, Language::Tsx, None).unwrap();
    assert_ne!(h1, h2);
}

#[test]
fn xml_fingerprint_detects_structural_changes() {
    let source = std::fs::read_to_string("tests/fixtures/form.xml").unwrap();
    let h1 = ast_fingerprint(&source, Language::Xml, None).unwrap();
    // Add a new element — structural change the parser will detect
    let modified = source.replace(
        "</form>",
        "  <field name=\"confirm\" type=\"password\" required=\"true\" />\n</form>",
    );
    let h2 = ast_fingerprint(&modified, Language::Xml, None).unwrap();
    assert_ne!(h1, h2);
}

#[test]
fn compute_sig_produces_prefixed_output() {
    let sig = compute_sig("public class Foo {}", "Foo.java", None);
    assert!(sig.starts_with(SIG_PREFIX));
    // sig: + 16 hex chars = 20 chars total
    assert_eq!(sig.len(), 20);
}

#[test]
fn compute_sig_is_consistent() {
    let s1 = compute_sig("public class Foo {}", "Foo.java", None);
    let s2 = compute_sig("public class Foo {}", "Foo.java", None);
    assert_eq!(s1, s2);
}

#[test]
fn compute_sig_differs_for_changes() {
    let s1 = compute_sig("public class Foo {}", "Foo.java", None);
    let s2 = compute_sig("public class Foo { int x; }", "Foo.java", None);
    assert_ne!(s1, s2);
}

#[test]
fn compute_sig_uses_content_hash_for_unknown_extensions() {
    let s1 = compute_sig("hello world", "unknown.xyz", None);
    assert!(s1.starts_with(SIG_PREFIX));
    let s2 = compute_sig("hello world", "unknown.xyz", None);
    assert_eq!(s1, s2);
}

// --- Python ---

#[test]
fn python_fingerprint_ignores_whitespace() {
    let compact = "def foo():\n  return 1";
    let spaced = "def foo():\n    return 1";
    let h1 = ast_fingerprint(compact, Language::Python, None).unwrap();
    let h2 = ast_fingerprint(spaced, Language::Python, None).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn python_fingerprint_ignores_comments() {
    let before = "def foo():\n    return 1";
    let after = "# this is a comment\ndef foo():\n    return 1";
    let h1 = ast_fingerprint(before, Language::Python, None).unwrap();
    let h2 = ast_fingerprint(after, Language::Python, None).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn python_fingerprint_detects_changes() {
    let source = std::fs::read_to_string("tests/fixtures/auth.py").unwrap();
    let h1 = ast_fingerprint(&source, Language::Python, None).unwrap();
    let modified = source.replace("return False", "raise ValueError('invalid')");
    let h2 = ast_fingerprint(&modified, Language::Python, None).unwrap();
    assert_ne!(h1, h2);
}

#[test]
fn python_symbol_scoped_class_method() {
    let source = std::fs::read_to_string("tests/fixtures/auth.py").unwrap();
    let full = ast_fingerprint(&source, Language::Python, None).unwrap();
    let method = ast_fingerprint(
        &source,
        Language::Python,
        Some("AuthService#validate_token"),
    )
    .unwrap();
    assert_ne!(full, method);
    // Consistent
    let method2 = ast_fingerprint(
        &source,
        Language::Python,
        Some("AuthService#validate_token"),
    )
    .unwrap();
    assert_eq!(method, method2);
}

#[test]
fn python_symbol_scoped_standalone_function() {
    let source = std::fs::read_to_string("tests/fixtures/auth.py").unwrap();
    let full = ast_fingerprint(&source, Language::Python, None).unwrap();
    let func = ast_fingerprint(&source, Language::Python, Some("standalone_function")).unwrap();
    assert_ne!(full, func);
}

#[test]
fn python_symbol_scoped_decorated_method() {
    let source = std::fs::read_to_string("tests/fixtures/auth.py").unwrap();
    let method = ast_fingerprint(
        &source,
        Language::Python,
        Some("AuthService#refresh_session"),
    )
    .unwrap();
    assert!(!method.is_empty());
}

// --- Rust ---

#[test]
fn rust_fingerprint_ignores_whitespace() {
    let compact = "fn foo() -> i32 { 42 }";
    let spaced = "fn foo() -> i32 {\n    42\n}";
    let h1 = ast_fingerprint(compact, Language::Rust, None).unwrap();
    let h2 = ast_fingerprint(spaced, Language::Rust, None).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn rust_fingerprint_ignores_comments() {
    let before = "fn foo() -> i32 { 42 }";
    let after = "/// Doc comment\n// line comment\nfn foo() -> i32 { 42 }";
    let h1 = ast_fingerprint(before, Language::Rust, None).unwrap();
    let h2 = ast_fingerprint(after, Language::Rust, None).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn rust_fingerprint_detects_changes() {
    let source = std::fs::read_to_string("tests/fixtures/handler.rs").unwrap();
    let h1 = ast_fingerprint(&source, Language::Rust, None).unwrap();
    let modified = source.replace("x * 2", "x * 3");
    let h2 = ast_fingerprint(&modified, Language::Rust, None).unwrap();
    assert_ne!(h1, h2);
}

#[test]
fn rust_symbol_scoped_impl_method() {
    let source = std::fs::read_to_string("tests/fixtures/handler.rs").unwrap();
    let full = ast_fingerprint(&source, Language::Rust, None).unwrap();
    let method =
        ast_fingerprint(&source, Language::Rust, Some("AuthHandler#validate_token")).unwrap();
    assert_ne!(full, method);
    // Consistent
    let method2 =
        ast_fingerprint(&source, Language::Rust, Some("AuthHandler#validate_token")).unwrap();
    assert_eq!(method, method2);
}

#[test]
fn rust_symbol_scoped_standalone_function() {
    let source = std::fs::read_to_string("tests/fixtures/handler.rs").unwrap();
    let full = ast_fingerprint(&source, Language::Rust, None).unwrap();
    let func = ast_fingerprint(&source, Language::Rust, Some("standalone_function")).unwrap();
    assert_ne!(full, func);
}

#[test]
fn rust_compute_sig_works() {
    let sig = compute_sig("fn main() {}", "main.rs", None);
    assert!(sig.starts_with(SIG_PREFIX));
    assert_eq!(sig.len(), 20);
}

#[test]
fn python_compute_sig_works() {
    let sig = compute_sig("def main(): pass", "main.py", None);
    assert!(sig.starts_with(SIG_PREFIX));
    assert_eq!(sig.len(), 20);
}
