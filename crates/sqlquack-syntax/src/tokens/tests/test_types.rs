use crate::tokens::errors::Error;
use crate::tokens::models::DialectType;
use crate::tokens::models::TokenType;
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

pub(crate) struct ErrorPositionTestCase {
    pub(crate) description: &'static str,
    pub(crate) error: Error,
    pub(crate) expected_position: (Option<usize>, Option<usize>, Option<usize>, Option<usize>),
    pub(crate) expected_message_parts: &'static [&'static str],
}

pub(crate) struct ValidationLocationTestCase {
    pub(crate) description: &'static str,
    pub(crate) location: (usize, usize),
    pub(crate) span: (Option<usize>, Option<usize>),
    pub(crate) expected_position: (Option<usize>, Option<usize>, Option<usize>, Option<usize>),
}

pub(crate) struct DialectNameTestCase {
    pub(crate) description: &'static str,
    pub(crate) name: &'static str,
    pub(crate) expected_dialect: Option<DialectType>,
}
