//! Guard: defaults.

use crate::guard::constants::DEFAULT_MAX_AST_DEPTH;
use crate::guard::constants::DEFAULT_MAX_AST_NODES;
use crate::guard::constants::DEFAULT_MAX_FUNCTION_CALL_DEPTH;
use crate::guard::constants::DEFAULT_MAX_INPUT_BYTES;
use crate::guard::constants::DEFAULT_MAX_PARENTHESES_DEPTH;
use crate::guard::constants::DEFAULT_MAX_PARSER_DEPTH;
use crate::guard::constants::DEFAULT_MAX_TOKENS;

pub(crate) fn default_max_parser_depth() -> Option<usize> {
    Some(DEFAULT_MAX_PARSER_DEPTH)
}

pub(crate) fn default_max_input_bytes() -> Option<usize> {
    Some(DEFAULT_MAX_INPUT_BYTES)
}

pub(crate) fn default_max_tokens() -> Option<usize> {
    Some(DEFAULT_MAX_TOKENS)
}

pub(crate) fn default_max_ast_nodes() -> Option<usize> {
    Some(DEFAULT_MAX_AST_NODES)
}

pub(crate) fn default_max_ast_depth() -> Option<usize> {
    Some(DEFAULT_MAX_AST_DEPTH)
}

pub(crate) fn default_max_parenthesis_depth() -> Option<usize> {
    Some(DEFAULT_MAX_PARENTHESES_DEPTH)
}

pub(crate) fn default_max_function_call_depth() -> Option<usize> {
    Some(DEFAULT_MAX_FUNCTION_CALL_DEPTH)
}
