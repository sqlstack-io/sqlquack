use std::sync::Arc;

use crate::tokens::models::ParserTokenText;
use crate::tokens::models::Token;
use crate::tokens::models::TokenType;
use crate::tokens::models::Tokenizer;
use crate::tokens::models::TokenizerConfig;

pub(crate) fn backslash_escapes(follow: &[char]) -> TokenizerConfig {
    let mut config = TokenizerConfig::default();
    config.string_escapes.push('\\');
    config.escape_follow_chars = follow.to_vec();
    config
}

pub(crate) fn terminal_backslash_recovery() -> TokenizerConfig {
    let mut config = TokenizerConfig::default();
    config.string_escapes.push('\\');
    config.recover_terminal_backslash_quote = true;
    config
}

pub(crate) fn underscore_numbers() -> TokenizerConfig {
    TokenizerConfig {
        numbers_can_be_underscore_separated: true,
        ..TokenizerConfig::default()
    }
}

pub(crate) fn typed_texts(
    config: TokenizerConfig,
    sql: &str,
) -> Result<Vec<(TokenType, String)>, String> {
    Tokenizer::new(config)
        .tokenize(sql)
        .map(|tokens| {
            tokens
                .into_iter()
                .map(|token| (token.token_type, token.text))
                .collect()
        })
        .map_err(|error| error.to_string())
}

pub(crate) fn trailing_comments(sql: &str) -> Vec<Vec<String>> {
    Tokenizer::default()
        .tokenize(sql)
        .expect("the query tokenizes")
        .into_iter()
        .map(|token| token.trailing_comments)
        .collect()
}

pub(crate) fn fast_path_matches_character_path(sql: &str) -> bool {
    let tokenizer = Tokenizer::default();
    tokenizer.tokenize(sql).expect("the query tokenizes")
        == tokenizer
            .tokenize_without_ascii_fast_path(sql)
            .expect("the query tokenizes without the fast path")
}

pub(crate) fn parser_tokens_match_public_tokens(sql: &str) -> bool {
    let tokenizer = Tokenizer::default();
    let public = tokenizer.tokenize(sql).expect("the query tokenizes");
    let source: Arc<str> = Arc::from(sql);
    let (parser, stats) = tokenizer
        .tokenize_for_parser(&source)
        .expect("the query tokenizes for the parser");
    let materialized: Vec<Token> = parser
        .iter()
        .map(|token| Token {
            token_type: token.token_type,
            text: token.text_owned(),
            span: token.span,
            comments: token.comments.clone(),
            trailing_comments: token.trailing_comments.clone(),
        })
        .collect();
    materialized == public && stats.token_count == public.len()
}

pub(crate) fn parser_tokens_borrow_source(sql: &str) -> bool {
    let source: Arc<str> = Arc::from(sql);
    let (tokens, _) = Tokenizer::default()
        .tokenize_for_parser(&source)
        .expect("the query tokenizes for the parser");
    tokens
        .iter()
        .all(|token| matches!(&token.text, ParserTokenText::Source { .. }))
}
