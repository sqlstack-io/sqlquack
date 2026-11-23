//! The tokenizer, its configuration, and the cursors and state it scans with.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use sqlquack_syntax::guard::models::TokenGuardStats;
use sqlquack_syntax::tokens::models::{ParserToken, ParserTokenText, Span, Token, TokenType};
use sqlquack_syntax::tokens::types::Result;

use crate::tokens::_helpers::defaults::DEFAULT_COMMENTS;
use crate::tokens::_helpers::defaults::DEFAULT_IDENTIFIERS;
use crate::tokens::_helpers::defaults::DEFAULT_KEYWORDS;
use crate::tokens::_helpers::defaults::DEFAULT_QUOTES;
use crate::tokens::_helpers::defaults::DEFAULT_SINGLE_TOKENS;
use crate::tokens::types::TokenOutput;
use crate::tokens::types::TokenizerCursor;

/// The SQL a token's text comes from, with the shared copy parser tokens point into.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TokenSource<'a> {
    pub(crate) text: &'a str,
    pub(crate) shared: Option<&'a Arc<str>>,
}

/// Tokenizer configuration for a dialect
#[derive(Debug, Clone)]
pub struct TokenizerConfig {
    /// Keywords mapping (uppercase keyword -> token type)
    pub keywords: HashMap<String, TokenType>,
    /// Single character tokens
    pub single_tokens: HashMap<char, TokenType>,
    /// Quote characters (start -> end)
    pub quotes: HashMap<String, String>,
    /// Identifier quote characters (start -> end)
    pub identifiers: HashMap<char, char>,
    /// Whether quoted identifiers use string-style backslash escapes (BigQuery/ClickHouse).
    pub identifier_backslash_escapes: bool,
    /// Comment definitions (start -> optional end)
    pub comments: HashMap<String, Option<String>>,
    /// String escape characters
    pub string_escapes: Vec<char>,
    /// Whether to support nested comments
    pub nested_comments: bool,
    /// Characters a backslash escapes (MySQL); otherwise the backslash is dropped.
    pub escape_follow_chars: Vec<char>,
    /// Whether b'...' is a byte string (BigQuery) rather than a bit string (the default).
    pub b_prefix_is_byte_string: bool,
    /// Numeric literal suffixes and their types, such as `L` for BIGINT (Hive, Spark).
    pub numeric_literals: HashMap<String, String>,
    /// Whether unquoted identifiers may start with a digit, as `1a` (Hive, Spark, MySQL).
    pub identifiers_can_start_with_digit: bool,
    /// Whether a 0x prefix starts a hex literal (BigQuery, SQLite, Teradata).
    pub hex_number_strings: bool,
    /// Whether 0x literals are integers (BigQuery) rather than binary strings.
    pub hex_string_is_integer_type: bool,
    /// Whether an empty `0x` is an empty binary string (TSQL).
    pub allow_empty_hex_string: bool,
    /// Whether escapes such as \' work in raw strings (BigQuery) or not (Spark).
    pub string_escapes_allowed_in_raw_strings: bool,
    /// Whether # starts a single-line comment (ClickHouse, MySQL)
    pub hash_comments: bool,
    /// Whether `$` can start or continue an identifier (ClickHouse).
    pub dollar_sign_is_identifier: bool,
    /// Whether data after INSERT ... FORMAT name is raw up to a blank line (ClickHouse).
    pub insert_format_raw_data: bool,
    /// Whether numbers may use `_` digit separators, as `1_000` (ClickHouse, DuckDB).
    pub numbers_can_be_underscore_separated: bool,
    /// Recovers `'a\' or 1=1` by closing at the escaped quote when no later quote exists.
    pub recover_terminal_backslash_quote: bool,
    /// Recovers an unterminated final single-quoted string at end of input (ClickHouse).
    pub recover_unterminated_string: bool,
    /// Whether `//` is the integer-division operator (Vertica).
    pub double_slash_int_div: bool,
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            keywords: DEFAULT_KEYWORDS.clone(),
            single_tokens: DEFAULT_SINGLE_TOKENS.clone(),
            quotes: DEFAULT_QUOTES.clone(),
            identifiers: DEFAULT_IDENTIFIERS.clone(),
            identifier_backslash_escapes: false,
            comments: DEFAULT_COMMENTS.clone(),
            string_escapes: vec!['\''],
            nested_comments: true,
            escape_follow_chars: vec![],
            b_prefix_is_byte_string: false,
            numeric_literals: HashMap::new(),
            identifiers_can_start_with_digit: false,
            hex_number_strings: false,
            hex_string_is_integer_type: false,
            allow_empty_hex_string: false,
            string_escapes_allowed_in_raw_strings: true,
            hash_comments: false,
            dollar_sign_is_identifier: false,
            insert_format_raw_data: false,
            numbers_can_be_underscore_separated: false,
            recover_terminal_backslash_quote: false,
            recover_unterminated_string: false,
            double_slash_int_div: false,
        }
    }
}

/// SQL Tokenizer
#[derive(Debug)]
pub struct Tokenizer {
    pub(crate) config: Arc<TokenizerConfig>,
}

impl Tokenizer {
    /// Whether this tokenizer's block comments nest, as lossless trivia must read them.
    pub fn nested_comments(&self) -> bool {
        self.config.nested_comments
    }

    /// Create a new tokenizer with the given configuration
    pub fn new(config: TokenizerConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub fn from_shared_config(config: Arc<TokenizerConfig>) -> Self {
        Self { config }
    }

    /// Create a tokenizer with default configuration
    pub fn default_config() -> Self {
        Self::new(TokenizerConfig::default())
    }

    /// Tokenize a SQL string
    pub fn tokenize(&self, sql: &str) -> Result<Vec<Token>> {
        if sql.is_ascii() {
            TokenizerState::<_, Token>::new(sql, &self.config, AsciiCursor(sql.as_bytes()))
                .tokenize()
        } else {
            TokenizerState::<_, Token>::new(sql, &self.config, UnicodeCursor::new(sql)).tokenize()
        }
    }

    pub fn tokenize_for_parser(
        &self,
        sql: &Arc<str>,
    ) -> Result<(Vec<ParserToken>, TokenGuardStats)> {
        if sql.is_ascii() {
            let mut state = TokenizerState::<_, ParserToken>::new_shared(
                sql,
                Arc::clone(sql),
                &self.config,
                AsciiCursor(sql.as_bytes()),
            );
            let tokens = state.tokenize()?;
            Ok((tokens, state.guard_stats.take().unwrap_or_default()))
        } else {
            let mut state = TokenizerState::<_, ParserToken>::new_shared(
                sql,
                Arc::clone(sql),
                &self.config,
                UnicodeCursor::new(sql),
            );
            let tokens = state.tokenize()?;
            Ok((tokens, state.guard_stats.take().unwrap_or_default()))
        }
    }

    #[cfg(test)]
    pub(crate) fn tokenize_without_ascii_fast_path(&self, sql: &str) -> Result<Vec<Token>> {
        TokenizerState::new(sql, &self.config, UnicodeCursor::new(sql)).tokenize()
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::default_config()
    }
}

#[derive(Debug)]
pub(crate) struct AsciiCursor<'a>(pub(crate) &'a [u8]);

impl TokenizerCursor for AsciiCursor<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn char_at(&self, index: usize) -> char {
        self.0[index] as char
    }

    #[inline]
    fn text_from_range(&self, source: &str, start: usize, end: usize) -> String {
        source[start..end].to_string()
    }

    #[inline]
    fn source_range<'a>(&self, source: &'a str, start: usize, end: usize) -> Option<&'a str> {
        Some(&source[start..end])
    }
}

#[derive(Debug)]
pub(crate) struct UnicodeCursor(pub(crate) Vec<char>);

impl UnicodeCursor {
    pub(crate) fn new(source: &str) -> Self {
        Self(source.chars().collect())
    }
}

impl TokenizerCursor for UnicodeCursor {
    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    fn char_at(&self, index: usize) -> char {
        self.0[index]
    }

    #[inline]
    fn text_from_range(&self, _source: &str, start: usize, end: usize) -> String {
        self.0[start..end].iter().collect()
    }
}

/// Internal state for tokenization
#[derive(Debug)]
pub(crate) struct TokenizerState<'a, C, T> {
    pub(crate) source: &'a str,
    pub(crate) shared_source: Option<Arc<str>>,
    pub(crate) cursor: C,
    pub(crate) size: usize,
    pub(crate) tokens: Vec<T>,
    pub(crate) start: usize,
    pub(crate) current: usize,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) comments: Vec<String>,
    pub(crate) guard_stats: Option<TokenGuardStats>,
    pub(crate) config: &'a TokenizerConfig,
    pub(crate) ascii: AsciiLookups,
}

/// The config's single-character tokens and identifier quotes for ASCII characters, read without hashing.
#[derive(Debug)]
pub(crate) struct AsciiLookups {
    pub(crate) single_tokens: [Option<TokenType>; 128],
    pub(crate) identifiers: [Option<char>; 128],
}

impl TokenOutput for Token {
    fn from_source(
        token_type: TokenType,
        source: TokenSource<'_>,
        text: Range<usize>,
        span: Span,
    ) -> Self {
        Self::new(token_type, &source.text[text], span)
    }

    fn from_owned(token_type: TokenType, text: String, span: Span) -> Self {
        Self::new(token_type, text, span)
    }

    fn token_type(&self) -> TokenType {
        self.token_type
    }

    fn text<'a>(&'a self, _source: &'a str) -> &'a str {
        &self.text
    }

    fn comments_mut(&mut self) -> &mut Vec<String> {
        &mut self.comments
    }

    fn trailing_comments_mut(&mut self) -> &mut Vec<String> {
        &mut self.trailing_comments
    }
}

impl TokenOutput for ParserToken {
    fn from_source(
        token_type: TokenType,
        source: TokenSource<'_>,
        text: Range<usize>,
        span: Span,
    ) -> Self {
        let text = match source.shared {
            Some(shared) => ParserTokenText::Source {
                source: Arc::clone(shared),
                start: text.start,
                end: text.end,
            },
            None => ParserTokenText::Owned(source.text[text].to_string()),
        };
        Self {
            token_type,
            span,
            comments: Vec::new(),
            trailing_comments: Vec::new(),
            text,
        }
    }

    fn from_owned(token_type: TokenType, text: String, span: Span) -> Self {
        Self {
            token_type,
            span,
            comments: Vec::new(),
            trailing_comments: Vec::new(),
            text: ParserTokenText::Owned(text),
        }
    }

    fn token_type(&self) -> TokenType {
        self.token_type
    }

    fn text<'a>(&'a self, _source: &'a str) -> &'a str {
        self.text()
    }

    fn comments_mut(&mut self) -> &mut Vec<String> {
        &mut self.comments
    }

    fn trailing_comments_mut(&mut self) -> &mut Vec<String> {
        &mut self.trailing_comments
    }
}
