use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

/// Supported languages for AST fingerprinting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Java,
    Go,
    TypeScript,
    Xml,
}

impl Language {
    /// Map a file extension to a language, if supported.
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext {
            "java" => Some(Language::Java),
            "go" => Some(Language::Go),
            "ts" | "tsx" | "js" | "jsx" => Some(Language::TypeScript),
            "xml" => Some(Language::Xml),
            _ => None,
        }
    }

    /// Return the tree-sitter grammar for this language.
    fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Language::Java => tree_sitter_java::LANGUAGE.into(),
            Language::Go => tree_sitter_go::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Xml => tree_sitter_xml::LANGUAGE_XML.into(),
        }
    }
}

/// SHA-256 hash of raw content (fallback for unsupported languages).
pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// AST-based fingerprint that is insensitive to whitespace and comment changes.
///
/// If `symbol` is provided (e.g. `"ClassName#methodName"`), only the matching
/// AST subtree is hashed. Returns the hex-encoded SHA-256 digest.
pub fn ast_fingerprint(source: &str, lang: Language, symbol: Option<&str>) -> Result<String> {
    let ts_lang = lang.tree_sitter_language();
    let mut parser = Parser::new();
    parser
        .set_language(&ts_lang)
        .context("failed to set tree-sitter language")?;

    let tree = parser
        .parse(source, None)
        .context("tree-sitter parse returned None")?;

    let root = tree.root_node();

    let target = match symbol {
        Some(sym) => find_symbol_node(root, source, sym, lang)
            .with_context(|| format!("symbol not found: {sym}"))?,
        None => root,
    };

    let mut hasher = Sha256::new();
    walk_and_hash(target, source, &mut hasher, lang);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Recursively walk the AST, hashing node kinds and leaf token text.
/// Skips comment nodes entirely.
fn walk_and_hash(node: Node, source: &str, hasher: &mut Sha256, lang: Language) {
    if is_comment(node.kind(), lang) {
        return;
    }

    // Hash the structural node kind.
    hasher.update(node.kind().as_bytes());

    if node.child_count() == 0 {
        // Leaf node: hash the actual token text (but not whitespace-only tokens).
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            if !text.trim().is_empty() {
                hasher.update(text.as_bytes());
            }
        }
    } else {
        // Recurse into children using index-based access.
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                walk_and_hash(child, source, hasher, lang);
            }
        }
    }
}

/// Check whether a node kind represents a comment.
fn is_comment(kind: &str, _lang: Language) -> bool {
    matches!(
        kind,
        "line_comment" | "block_comment" | "comment" | "multiline_comment"
    )
}

/// Find an AST node matching a symbol spec like `"ClassName#methodName"`.
fn find_symbol_node<'tree>(
    root: Node<'tree>,
    source: &str,
    symbol: &str,
    lang: Language,
) -> Option<Node<'tree>> {
    let (class_name, method_name) = match symbol.split_once('#') {
        Some((c, m)) => (Some(c), m),
        None => (None, symbol),
    };

    find_symbol_recursive(root, source, class_name, method_name, lang)
}

/// Recursively search for a matching class/method declaration.
fn find_symbol_recursive<'tree>(
    node: Node<'tree>,
    source: &str,
    class_name: Option<&str>,
    method_name: &str,
    lang: Language,
) -> Option<Node<'tree>> {
    match lang {
        Language::Java | Language::TypeScript => {
            if is_class_declaration(node.kind(), lang) {
                if let Some(target_class) = class_name {
                    let name = node_field_text(node, "name", source)?;
                    if name == target_class {
                        return find_method_in_class(node, source, method_name, lang);
                    }
                }
            }

            if class_name.is_none() && is_method_declaration(node.kind(), lang) {
                let name = node_field_text(node, "name", source)?;
                if name == method_name {
                    return Some(node);
                }
            }
        }
        Language::Go => {
            if is_method_declaration(node.kind(), lang) {
                let name = node_field_text(node, "name", source)?;
                if name == method_name {
                    return Some(node);
                }
            }
        }
        Language::Xml => {
            // XML doesn't have class/method concepts
            return None;
        }
    }

    // Recurse into children.
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if let Some(found) =
                find_symbol_recursive(child, source, class_name, method_name, lang)
            {
                return Some(found);
            }
        }
    }
    None
}

/// Find a method node inside a class body.
fn find_method_in_class<'tree>(
    class_node: Node<'tree>,
    source: &str,
    method_name: &str,
    lang: Language,
) -> Option<Node<'tree>> {
    // Look in the class body for method declarations.
    if let Some(body) = class_node.child_by_field_name("body") {
        for i in 0..body.child_count() {
            if let Some(child) = body.child(i) {
                if is_method_declaration(child.kind(), lang) {
                    if let Some(name) = node_field_text(child, "name", source) {
                        if name == method_name {
                            return Some(child);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Check if a node kind is a class declaration.
fn is_class_declaration(kind: &str, _lang: Language) -> bool {
    matches!(kind, "class_declaration" | "class")
}

/// Check if a node kind is a method/function declaration.
fn is_method_declaration(kind: &str, lang: Language) -> bool {
    match lang {
        Language::Java => matches!(kind, "method_declaration" | "constructor_declaration"),
        Language::Go => matches!(
            kind,
            "function_declaration" | "method_declaration"
        ),
        Language::TypeScript => matches!(
            kind,
            "method_definition" | "function_declaration" | "function"
        ),
        Language::Xml => false,
    }
}

/// Extract the text of a named field from a node.
fn node_field_text<'a>(node: Node, field: &str, source: &'a str) -> Option<&'a str> {
    let child = node.child_by_field_name(field)?;
    child.utf8_text(source.as_bytes()).ok()
}
