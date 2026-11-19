//! The one tokenize-and-parse step every lint request shares.

use std::collections::HashSet;

use sqlquack_parse::parsing::constants::{MAX_CALL_DEPTH_CEILING, MAX_FUNCTION_CALL_DEPTH};
use sqlquack_parse::parsing::main::call_depth_hint::call_depth_hint;
use sqlquack_parse::parsing::main::checked_call_depth::checked_call_depth;
use sqlquack_syntax::guard::main::enforce_lexed_tokens::enforce_lexed_tokens;
use sqlquack_syntax::tokens::models::{Token, TokenType};
use sqlquack_syntax::{ComplexityGuardOptions, Expression, ExpressionWalk, ParseOptions};

use crate::dialects::main::profile::profile;
use crate::dialects::models::LintProfile;
use crate::lint::models::{LintDialect, ParsedSql};

/// Tokenize and parse `sql`, reading a harness's `FROM <empty_relation>()` as a bare relation.
pub(crate) fn parse(
    sql: &str,
    dialect: LintDialect,
    (empty_relation, max_call_depth): (Option<&str>, Option<usize>),
) -> Result<ParsedSql, String> {
    let max_call_depth: usize =
        checked_call_depth(max_call_depth.unwrap_or(MAX_FUNCTION_CALL_DEPTH))
            .map_err(|reason| format!("max_function_call_depth: {reason}"))?;
    let profile: LintProfile = profile(dialect);
    let tokens: Vec<Token> = (profile.tokens)(sql)?;
    let depth_guard = ComplexityGuardOptions {
        max_parser_depth: None,
        max_input_bytes: None,
        max_tokens: None,
        max_ast_nodes: None,
        max_ast_depth: None,
        max_parenthesis_depth: Some(MAX_CALL_DEPTH_CEILING),
        max_function_call_depth: Some(max_call_depth),
    };
    enforce_lexed_tokens(&tokens, &depth_guard).map_err(|error| {
        call_depth_hint(
            error.to_string(),
            max_call_depth,
            Some("max_function_call_depth"),
        )
    })?;
    let options = ParseOptions {
        complexity_guard: Some(ComplexityGuardOptions {
            max_function_call_depth: Some(max_call_depth),
            ..Default::default()
        }),
    };
    let parse_text = empty_relation
        .map(|name| without_empty_relation_parentheses(sql, &tokens, name))
        .unwrap_or_default();
    let parse_text = if parse_text.is_empty() {
        sql
    } else {
        &parse_text
    };
    let statements: Vec<Expression> =
        (profile.statements)(parse_text, &options).map_err(|message| {
            call_depth_hint(message, max_call_depth, Some("max_function_call_depth"))
        })?;
    for statement in &statements {
        if statement
            .dfs()
            .any(|node| matches!(node, Expression::Command(_) | Expression::Raw(_)))
        {
            return Err("unsupported syntax: native parser retained an opaque SQL node".to_owned());
        }
    }
    Ok(ParsedSql {
        dialect,
        tokens,
        statements,
    })
}

/// The expressions starting a predicate `text` (tokenized as `tokens`), ignoring what follows.
pub(crate) fn predicate_expressions(
    text: &str,
    tokens: &[Token],
    dialect: LintDialect,
) -> Result<Vec<Expression>, String> {
    (profile(dialect).predicates)(text, tokens)
}

/// `sql` with each `FROM <empty_relation> ( )` blanked to spaces past Snowflake's `TABLE( )` rule; empty when none.
fn without_empty_relation_parentheses(sql: &str, tokens: &[Token], empty_relation: &str) -> String {
    let significant: Vec<&Token> = tokens
        .iter()
        .filter(|token| {
            !matches!(
                token.token_type,
                TokenType::Space
                    | TokenType::Break
                    | TokenType::LineComment
                    | TokenType::BlockComment
            )
        })
        .collect();
    let blanked: HashSet<usize> = significant
        .windows(4)
        .filter(|window| {
            window[0].text.eq_ignore_ascii_case("from")
                && window[1].text.eq_ignore_ascii_case(empty_relation)
                && window[2].token_type == TokenType::LParen
                && window[3].token_type == TokenType::RParen
        })
        .flat_map(|window| [window[2].span.start, window[3].span.start])
        .collect();
    if blanked.is_empty() {
        return String::new();
    }
    sql.chars()
        .enumerate()
        .map(|(offset, character)| {
            if blanked.contains(&offset) {
                ' '
            } else {
                character
            }
        })
        .collect()
}
