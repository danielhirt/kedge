use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Java,
    Go,
    TypeScript,
    Tsx,
    Xml,
    Python,
    Rust,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext {
            "java" => Some(Language::Java),
            "go" => Some(Language::Go),
            "ts" => Some(Language::TypeScript),
            "tsx" | "jsx" => Some(Language::Tsx),
            "js" => Some(Language::TypeScript),
            "xml" => Some(Language::Xml),
            "py" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            _ => None,
        }
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        match self {
            Language::Java => tree_sitter_java::LANGUAGE.into(),
            Language::Go => tree_sitter_go::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Language::Xml => tree_sitter_xml::LANGUAGE_XML.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        }
    }
}

pub const SIG_PREFIX: &str = "sig:";

/// Returns `sig:<hex>` — AST fingerprint for supported languages, content hash otherwise.
pub fn compute_sig(content: &str, path: &str, symbol: Option<&str>) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let hash = match Language::from_extension(ext) {
        Some(lang) => {
            ast_fingerprint(content, lang, symbol).unwrap_or_else(|_| content_hash(content))
        }
        None => content_hash(content),
    };

    // Truncate to 16 hex chars for readability (64 bits — sufficient for drift detection)
    format!("{SIG_PREFIX}{}", &hash[..16])
}

pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Whitespace/comment-insensitive fingerprint. If `symbol` is `"Class#method"`,
/// only that subtree is hashed.
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

fn walk_and_hash(node: Node, source: &str, hasher: &mut Sha256, lang: Language) {
    if is_comment(node.kind(), lang) {
        return;
    }

    hasher.update(node.kind().as_bytes());

    if node.child_count() == 0 {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            if !text.trim().is_empty() {
                hasher.update(text.as_bytes());
            }
        }
    } else {
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                walk_and_hash(child, source, hasher, lang);
            }
        }
    }
}

fn is_comment(kind: &str, _lang: Language) -> bool {
    matches!(
        kind,
        "line_comment" | "block_comment" | "comment" | "multiline_comment"
    )
}

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

fn find_symbol_recursive<'tree>(
    node: Node<'tree>,
    source: &str,
    class_name: Option<&str>,
    method_name: &str,
    lang: Language,
) -> Option<Node<'tree>> {
    match lang {
        Language::Java | Language::TypeScript | Language::Tsx | Language::Python => {
            let inner = if lang == Language::Python && node.kind() == "decorated_definition" {
                node.child_by_field_name("definition")
            } else {
                None
            };
            let effective = inner.unwrap_or(node);

            if is_class_declaration(effective.kind(), lang) {
                if let Some(target_class) = class_name {
                    let name = node_field_text(effective, "name", source)?;
                    if name == target_class {
                        return find_method_in_class(effective, source, method_name, lang);
                    }
                }
            }

            if class_name.is_none() && is_method_declaration(effective.kind(), lang) {
                let name = node_field_text(effective, "name", source)?;
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
        Language::Rust => {
            if node.kind() == "impl_item" {
                if let Some(target_class) = class_name {
                    if let Some(type_node) = node.child_by_field_name("type") {
                        let type_name = type_node.utf8_text(source.as_bytes()).ok()?;
                        if type_name == target_class {
                            return find_method_in_class(node, source, method_name, lang);
                        }
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
        Language::Xml => {
            return None;
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if let Some(found) = find_symbol_recursive(child, source, class_name, method_name, lang)
            {
                return Some(found);
            }
        }
    }
    None
}

fn find_method_in_class<'tree>(
    class_node: Node<'tree>,
    source: &str,
    method_name: &str,
    lang: Language,
) -> Option<Node<'tree>> {
    if let Some(body) = class_node.child_by_field_name("body") {
        for i in 0..body.child_count() {
            if let Some(child) = body.child(i) {
                let effective =
                    if lang == Language::Python && child.kind() == "decorated_definition" {
                        child.child_by_field_name("definition").unwrap_or(child)
                    } else {
                        child
                    };
                if is_method_declaration(effective.kind(), lang) {
                    if let Some(name) = node_field_text(effective, "name", source) {
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

fn is_class_declaration(kind: &str, _lang: Language) -> bool {
    matches!(kind, "class_declaration" | "class" | "class_definition")
}

fn is_method_declaration(kind: &str, lang: Language) -> bool {
    match lang {
        Language::Java => matches!(kind, "method_declaration" | "constructor_declaration"),
        Language::Go => matches!(kind, "function_declaration" | "method_declaration"),
        Language::TypeScript | Language::Tsx => matches!(
            kind,
            "method_definition" | "function_declaration" | "function"
        ),
        Language::Python => matches!(kind, "function_definition"),
        Language::Rust => matches!(kind, "function_item"),
        Language::Xml => false,
    }
}

fn node_field_text<'a>(node: Node, field: &str, source: &'a str) -> Option<&'a str> {
    let child = node.child_by_field_name(field)?;
    child.utf8_text(source.as_bytes()).ok()
}
