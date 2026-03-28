// steer — Documentation drift detection and remediation
//
// Module structure (filled in by subsequent tasks):
// - config       : steer.toml parsing and configuration types

pub mod config;
pub mod detection;
pub mod frontmatter;
pub mod models;
pub mod triage;
pub mod remediation;
pub mod install;
// - models       : shared data types (reports, anchors, payloads)
// - frontmatter  : YAML frontmatter extraction from markdown
// - detection    : AST fingerprinting pipeline
// - triage       : AI-based drift severity classification
// - remediation  : agent invocation and MR creation
// - install      : doc repo cache management
