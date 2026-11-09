//! The one tokenize-and-parse step every lint request shares.

use std::collections::HashSet;

use sqlquack_syntax::tokens::models::{Token, TokenType};
use sqlquack_syntax::{ComplexityGuardOptions, Expression, ExpressionWalk, ParseOptions};

use crate::dialects::main::profile::profile;
use crate::dialects::models::LintProfile;
use crate::lint::models::{LintDialect, ParsedSql};

const MAX_LINT_FUNCTION_CALL_DEPTH: usize = 128;

/// Tokenize and parse `sql`, reading a harness's `FROM <empty_relation>()` as a bare relation.
pub(crate) fn parse(
    sql: &str,
    dialect: LintDialect,
    empty_relation: Option<&str>,
) -> Result<ParsedSql, String> {
    let profile: LintProfile = profile(dialect);
    let tokens: Vec<Token> = (profile.tokens)(sql)?;
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
    let statements: Vec<Expression> = (profile.statements)(parse_text, &options)?;
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
