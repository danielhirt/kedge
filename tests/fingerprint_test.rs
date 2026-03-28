use steer::detection::fingerprint::{ast_fingerprint, content_hash, Language};

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
    assert_eq!(Language::from_extension("py"), None);
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
