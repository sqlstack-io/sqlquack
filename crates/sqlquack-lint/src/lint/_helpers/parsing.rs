//! The one tokenize-and-parse step every lint request shares.

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::lint::models::LintDialect;
use sqlquack_syntax::tokens::models::{Token, TokenType, Tokenizer, TokenizerConfig};
use sqlquack_syntax::{
    ComplexityGuardOptions, Dialect, Expression, ExpressionWalk, ParseOptions, dialect_tokens,
    parse_expressions, parse_statements,
};

use crate::lint::constants::UNPARSED_DIALECTS;
use crate::lint::models::ParsedSql;

const MAX_LINT_FUNCTION_CALL_DEPTH: usize = 128;
const SELECT_PREFIX: &str = "SELECT ";

/// BigQuery's tokens as SQLClean read them: backtick names, three string quotes, `#` comments.
static BIGQUERY_TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
    let mut config: TokenizerConfig = TokenizerConfig::default();
    config.identifiers.remove(&'"');
    config.identifiers.insert('`', '`');
    config.identifier_backslash_escapes = true;
    for quote in ["\"", "'''", "\"\"\""] {
        config.quotes.insert(quote.to_owned(), quote.to_owned());
    }
    config.string_escapes = vec!['\'', '\\'];
    config.b_prefix_is_byte_string = true;
    config.hex_number_strings = true;
    config.hex_string_is_integer_type = true;
    config.hash_comments = true;
    Tokenizer::new(config)
});

/// The SQLQuack dialect `name` names; `None` for a known dialect SQLQuack does not parse yet.
pub(crate) fn dialect(name: &str) -> Result<Option<LintDialect>, String> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "snowflake" => Ok(Some(LintDialect::Snowflake)),
        "duckdb" => Ok(Some(LintDialect::DuckDB)),
        "sqlite" => Ok(Some(LintDialect::SQLite)),
        "postgres" | "postgresql" | "generic" | "" => Ok(Some(LintDialect::PostgreSQL)),
        "bigquery" => Ok(Some(LintDialect::BigQuery)),
        _ if UNPARSED_DIALECTS.contains(&lower.as_str()) => Ok(None),
        _ => Err(format!("Unknown dialect: {name}")),
    }
}

/// The sqlquack-syntax dialect that parses `dialect`; `None` when its own native crate does.
fn syntax_dialect(dialect: LintDialect) -> Option<Dialect> {
    match dialect {
        LintDialect::Snowflake => Some(Dialect::Snowflake),
        LintDialect::DuckDB => Some(Dialect::DuckDB),
        LintDialect::PostgreSQL => Some(Dialect::PostgreSQL),
        LintDialect::SQLite => Some(Dialect::SQLite),
        LintDialect::BigQuery => None,
    }
}

/// Tokenize and parse `sql`, reading a harness's `FROM <empty_relation>()` as a bare relation.
pub(crate) fn parse(
    sql: &str,
    dialect: LintDialect,
    empty_relation: Option<&str>,
) -> Result<ParsedSql, String> {
    let tokens: Vec<Token> = match syntax_dialect(dialect) {
        Some(syntax) => dialect_tokens(sql, syntax),
        None => BIGQUERY_TOKENIZER.tokenize(sql),
    }
    .map_err(|error| error.to_string())?;
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
    let statements: Vec<Expression> = match (dialect, syntax_dialect(dialect)) {
        (LintDialect::PostgreSQL, _) => postgres_statements(parse_text)?,
        (LintDialect::BigQuery, _) | (_, None) => bigquery_statements(parse_text)?,
        (_, Some(syntax)) => {
            parse_statements(parse_text, syntax, &options).map_err(|error| error.to_string())?
        }
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

/// The expressions starting a predicate `text` (tokenized as `tokens`), ignoring what follows.
pub(crate) fn predicate_expressions(
    text: &str,
    tokens: &[Token],
    dialect: LintDialect,
) -> Result<Vec<Expression>, String> {
    if let Some(syntax) = syntax_dialect(dialect) {
        return parse_expressions(tokens.to_vec(), syntax).map_err(|error| error.to_string());
    }
    let mut text: &str = text.trim_end();
    loop {
        let refused: usize =
            match sqlquack_syntax_bigquery::parse_strict(&format!("{SELECT_PREFIX}{text}")) {
                Ok(parsed) => {
                    return match parsed.statements.as_slice() {
                        [Expression::Select(select)] => Ok(select.expressions.clone()),
                        _ => Err("the predicate is not one expression list".to_owned()),
                    };
                }
                Err(sqlquack_syntax_bigquery::StrictFailure::Parse(error)) => error.position,
                Err(sqlquack_syntax_bigquery::StrictFailure::Uncovered { position }) => position,
            };
        let kept: &str = refused
            .checked_sub(SELECT_PREFIX.len())
            .filter(|&at| at > 0 && at < text.len())
            .and_then(|at| text.get(..at))
            .ok_or_else(|| "the predicate does not parse".to_owned())?;
        text = kept.trim_end();
    }
}

/// The statements of `sql` from SQLQuack's native PostgreSQL parser, refusing what PostgreSQL refuses.
fn postgres_statements(sql: &str) -> Result<Vec<Expression>, String> {
    use sqlquack_syntax_postgres::StrictFailure;
    match sqlquack_syntax_postgres::parse_strict(sql) {
        Ok(parsed) => Ok(parsed.statements),
        Err(StrictFailure::Parse(error)) => Err(parse_error(sql, error.position, &error.message)),
        Err(StrictFailure::Uncovered { position }) => Err(uncovered(sql, position)),
    }
}

/// The statements of `sql` from SQLQuack's native BigQuery parser, refusing what BigQuery refuses.
fn bigquery_statements(sql: &str) -> Result<Vec<Expression>, String> {
    use sqlquack_syntax_bigquery::StrictFailure;
    match sqlquack_syntax_bigquery::parse_strict(sql) {
        Ok(parsed) => Ok(parsed.statements),
        Err(StrictFailure::Parse(error)) => Err(parse_error(sql, error.position, &error.message)),
        Err(StrictFailure::Uncovered { position }) => Err(uncovered(sql, position)),
    }
}

/// The parse error for a character at byte `position` that no token of the dialect covers.
fn uncovered(sql: &str, position: usize) -> String {
    parse_error(
        sql,
        position,
        "no token of the dialect covers this character",
    )
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
