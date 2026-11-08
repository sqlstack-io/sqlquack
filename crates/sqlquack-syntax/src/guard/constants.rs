//! Constants of guard.

pub(crate) const DEFAULT_MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;

pub(crate) const DEFAULT_MAX_TOKENS: usize = 1_000_000;

pub(crate) const DEFAULT_MAX_AST_NODES: usize = 1_000_000;

pub(crate) const DEFAULT_MAX_AST_DEPTH: usize = 512;

pub(crate) const DEFAULT_MAX_PARENTHESES_DEPTH: usize = 512;

pub(crate) const DEFAULT_MAX_FUNCTION_CALL_DEPTH: usize = 64;

/// Deepest type nesting, every `[]`, `ARRAY<`, `ARRAY(`, `OBJECT(` or `MAP(` level counted as one.
pub(crate) const MAX_DATA_TYPE_DEPTH: usize = 512;

#[cfg(target_arch = "wasm32")]
pub(crate) const DEFAULT_MAX_PARSER_DEPTH: usize = 32;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) const DEFAULT_MAX_PARSER_DEPTH: usize = 1_024;
