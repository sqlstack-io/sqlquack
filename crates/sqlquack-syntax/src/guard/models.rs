//! Data models of guard.

use serde::Deserialize;
use serde::Serialize;

use crate::guard::_helpers::defaults::default_max_ast_depth;
use crate::guard::_helpers::defaults::default_max_ast_nodes;
use crate::guard::_helpers::defaults::default_max_function_call_depth;
use crate::guard::_helpers::defaults::default_max_input_bytes;
use crate::guard::_helpers::defaults::default_max_parenthesis_depth;
use crate::guard::_helpers::defaults::default_max_parser_depth;
use crate::guard::_helpers::defaults::default_max_tokens;
use crate::guard::_helpers::tokens::is_function_call_name_token;
use crate::guard::_helpers::tokens::is_trivia_token;
use crate::tokens::models::Span;
use crate::tokens::models::TokenType;

/// Per-call parsing options. An absent guard preserves dialect-specific defaults.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParseOptions {
    #[serde(default)]
    pub complexity_guard: Option<ComplexityGuardOptions>,
}

/// Complexity limits that turn very deep or large inputs into errors, not stack exhaustion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComplexityGuardOptions {
    /// Maximum nesting while parsing (32 on WASM, 1024 natively); raising it risks the stack.
    #[serde(default = "default_max_parser_depth")]
    pub max_parser_depth: Option<usize>,
    /// Maximum SQL input size in bytes; `None` disables the check.
    #[serde(default = "default_max_input_bytes")]
    pub max_input_bytes: Option<usize>,
    /// Maximum number of tokens after tokenizing; `None` disables the check.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: Option<usize>,
    /// Maximum AST node count after parsing; `None` disables the check.
    #[serde(default = "default_max_ast_nodes")]
    pub max_ast_nodes: Option<usize>,
    /// Maximum AST depth after parsing; `None` disables the check.
    #[serde(default = "default_max_ast_depth")]
    pub max_ast_depth: Option<usize>,
    /// Maximum parenthesis nesting before parsing; `None` disables the check.
    #[serde(default = "default_max_parenthesis_depth")]
    pub max_parenthesis_depth: Option<usize>,
    /// Maximum function-call nesting before parsing; `None` disables the check.
    #[serde(default = "default_max_function_call_depth")]
    pub max_function_call_depth: Option<usize>,
}

#[derive(Debug, Default)]
pub(crate) struct TokenGuardStats {
    pub token_count: usize,
    pub(crate) parenthesis_depth_spans: Vec<Span>,
    pub(crate) function_depth_spans: Vec<Span>,
    pub(crate) parenthesis_stack: Vec<bool>,
    pub(crate) parenthesis_depth: usize,
    pub(crate) function_depth: usize,
    pub(crate) previous_significant: Option<TokenType>,
}

impl TokenGuardStats {
    pub(crate) fn observe(&mut self, token_type: TokenType, span: Span) {
        self.token_count += 1;
        match token_type {
            TokenType::LParen => {
                self.parenthesis_depth += 1;
                if self.parenthesis_depth_spans.len() < self.parenthesis_depth {
                    self.parenthesis_depth_spans.push(span);
                }

                let is_function_call = self
                    .previous_significant
                    .map(is_function_call_name_token)
                    .unwrap_or(false);
                self.parenthesis_stack.push(is_function_call);
                if is_function_call {
                    self.function_depth += 1;
                    if self.function_depth_spans.len() < self.function_depth {
                        self.function_depth_spans.push(span);
                    }
                }
            }
            TokenType::RParen => {
                self.parenthesis_depth = self.parenthesis_depth.saturating_sub(1);
                if self.parenthesis_stack.pop().unwrap_or(false) {
                    self.function_depth = self.function_depth.saturating_sub(1);
                }
            }
            _ => {}
        }

        if !is_trivia_token(token_type) {
            self.previous_significant = Some(token_type);
        }
    }
}

impl Default for ComplexityGuardOptions {
    fn default() -> Self {
        Self {
            max_parser_depth: default_max_parser_depth(),
            max_input_bytes: default_max_input_bytes(),
            max_tokens: default_max_tokens(),
            max_ast_nodes: default_max_ast_nodes(),
            max_ast_depth: default_max_ast_depth(),
            max_parenthesis_depth: default_max_parenthesis_depth(),
            max_function_call_depth: default_max_function_call_depth(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct AstStats {
    pub(crate) node_count: usize,
    pub(crate) max_depth: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ConnectorKind {
    And,
    Or,
}
