//! The one tokenize-and-parse step every lint request shares.

use std::collections::HashSet;

use sqlquack_syntax::tokens::models::{Token, TokenType};
use sqlquack_syntax::{
    ComplexityGuardOptions, Dialect, Expression, ExpressionWalk, ParseOptions, dialect_tokens,
    parse_statements,
};

use crate::lint::constants::UNPARSED_DIALECTS;
use crate::lint::models::ParsedSql;

const MAX_LINT_FUNCTION_CALL_DEPTH: usize = 128;

/// The SQLQuack dialect `name` names; `None` for a known dialect SQLQuack does not parse yet.
pub(crate) fn dialect(name: &str) -> Result<Option<Dialect>, String> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "snowflake" => Ok(Some(Dialect::Snowflake)),
        "duckdb" => Ok(Some(Dialect::DuckDB)),
        _ if UNPARSED_DIALECTS.contains(&lower.as_str()) => Ok(None),
        _ => Err(format!("Unknown dialect: {name}")),
    }
}

/// Tokenize and parse `sql`, reading a harness's `FROM <empty_relation>()` as a bare relation.
pub(crate) fn parse(
    sql: &str,
    dialect: Dialect,
    empty_relation: Option<&str>,
) -> Result<ParsedSql, String> {
    let tokens = dialect_tokens(sql, dialect).map_err(|error| error.to_string())?;
    let options = ParseOptions {
        complexity_guard: Some(ComplexityGuardOptions {
            max_function_call_depth: Some(MAX_LINT_FUNCTION_CALL_DEPTH),
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
    let statements =
        parse_statements(parse_text, dialect, &options).map_err(|error| error.to_string())?;
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
