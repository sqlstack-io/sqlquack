//! The one tokenize-and-parse step every lint request shares.

use std::collections::HashSet;

use crate::lint::models::LintDialect;
use sqlquack_syntax::tokens::models::{Token, TokenType};
use sqlquack_syntax::{
    ComplexityGuardOptions, Dialect, Expression, ExpressionWalk, ParseOptions, dialect_tokens,
    parse_statements,
};

use sqlquack_syntax_postgres::StrictFailure;

use crate::lint::constants::UNPARSED_DIALECTS;
use crate::lint::models::ParsedSql;

const MAX_LINT_FUNCTION_CALL_DEPTH: usize = 128;

/// The SQLQuack dialect `name` names; `None` for a known dialect SQLQuack does not parse yet.
pub(crate) fn dialect(name: &str) -> Result<Option<LintDialect>, String> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "snowflake" => Ok(Some(LintDialect::Snowflake)),
        "duckdb" => Ok(Some(LintDialect::DuckDB)),
        "sqlite" => Ok(Some(LintDialect::SQLite)),
        "postgres" | "postgresql" | "generic" | "" => Ok(Some(LintDialect::PostgreSQL)),
        _ if UNPARSED_DIALECTS.contains(&lower.as_str()) => Ok(None),
        _ => Err(format!("Unknown dialect: {name}")),
    }
}

/// The sqlquack-syntax dialect whose tokenizer and expression parser read `dialect`.
pub(crate) fn syntax_dialect(dialect: LintDialect) -> Dialect {
    match dialect {
        LintDialect::Snowflake => Dialect::Snowflake,
        LintDialect::DuckDB => Dialect::DuckDB,
        LintDialect::PostgreSQL => Dialect::PostgreSQL,
        LintDialect::SQLite => Dialect::SQLite,
    }
}

/// Tokenize and parse `sql`, reading a harness's `FROM <empty_relation>()` as a bare relation.
pub(crate) fn parse(
    sql: &str,
    dialect: LintDialect,
    empty_relation: Option<&str>,
) -> Result<ParsedSql, String> {
    let tokens = dialect_tokens(sql, syntax_dialect(dialect)).map_err(|error| error.to_string())?;
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
    let statements: Vec<Expression> = match dialect {
        LintDialect::PostgreSQL => postgres_statements(parse_text)?,
        _ => parse_statements(parse_text, syntax_dialect(dialect), &options)
            .map_err(|error| error.to_string())?,
    };
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

/// The statements of `sql` from SQLQuack's native PostgreSQL parser, refusing what PostgreSQL refuses.
fn postgres_statements(sql: &str) -> Result<Vec<Expression>, String> {
    match sqlquack_syntax_postgres::parse_strict(sql) {
        Ok(parsed) => Ok(parsed.statements),
        Err(StrictFailure::Parse(error)) => Err(parse_error(sql, error.position, &error.message)),
        Err(StrictFailure::Uncovered { position }) => Err(parse_error(
            sql,
            position,
            "no PostgreSQL token covers this character",
        )),
    }
}

/// A parse error naming the one-based line and character column of byte `position`.
fn parse_error(sql: &str, position: usize, message: &str) -> String {
    let before: &str = sql.get(..position).unwrap_or(sql);
    let line: usize = before.matches('\n').count() + 1;
    let column: usize = before
        .rsplit('\n')
        .next()
        .map_or(0, |text| text.chars().count())
        + 1;
    format!("Parse error at line {line}, column {column}: {message}")
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
