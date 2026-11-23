use sqlquack_syntax::tokens::models::TokenType;

use crate::tokens::models::TokenizerConfig;

pub(crate) struct TokenizeTestCase {
    pub(crate) description: &'static str,
    pub(crate) config: TokenizerConfig,
    pub(crate) sql: &'static str,
    pub(crate) expected_tokens: Result<Vec<(TokenType, &'static str)>, &'static str>,
}

pub(crate) struct TokenPathTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) agrees: fn(&str) -> bool,
    pub(crate) expected_agreement: bool,
}

pub(crate) struct CommentTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_trailing_comments: Vec<Vec<&'static str>>,
}

pub(crate) struct GuardStatsTestCase {
    pub(crate) description: &'static str,
    pub(crate) options: sqlquack_syntax::ComplexityGuardOptions,
    pub(crate) expected_error: Result<(), String>,
}
