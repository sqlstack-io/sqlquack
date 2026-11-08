//! The current parser and the Git tree it was built from.

use std::process::Command;

use sqlquack_syntax::{CallMode, Dialect, Expression, ExpressionWalk, ParseOptions, Parsed};

use crate::check::models::{LocalFailure, LocalVerdict};
use crate::formats::constants::SNOWFLAKE_DIALECT;
use crate::formats::main::content_sha256::content_sha256;
use crate::formats::main::current_dialect::current_dialect;
use crate::formats::models::LocalParser;

/// The code prefix of every complexity-guard error, which is never a syntax verdict.
const GUARD_PREFIX: &str = "E_GUARD_";

/// An accepted parse; opaque when `gapped` or a Command or Raw node appears anywhere.
fn accepted(statements: &[Expression], gapped: bool) -> LocalVerdict {
    LocalVerdict::Accepted {
        opaque: gapped
            || statements.iter().any(|statement| {
                statement
                    .contains(|node| matches!(node, Expression::Command(_) | Expression::Raw(_)))
            }),
        tree_sha256: Some(match serde_json::to_string(&statements) {
            Ok(tree) => content_sha256(tree.as_bytes()),
            Err(error) => content_sha256(format!("unserialisable tree: {error}").as_bytes()),
        }),
    }
}

/// SQLQuack's Snowflake verdict; a Command or Raw node anywhere makes the parse opaque.
fn snowflake_verdict(sql: &str) -> LocalVerdict {
    match sqlquack_syntax::parse_statements_with(
        sql,
        Dialect::Snowflake,
        &ParseOptions::default(),
        CallMode::Faithful,
    ) {
        Ok(statements) => accepted(&statements, false),
        Err(sqlquack_syntax::Error::Parse { message, .. }) if message.starts_with(GUARD_PREFIX) => {
            LocalVerdict::Failed {
                failure: LocalFailure::Guard,
            }
        }
        Err(_) => LocalVerdict::Rejected,
    }
}

/// A native dialect parser's strict verdict: a gap makes it opaque, and any refusal rejects it.
fn native_verdict<E>(parsed: Result<Parsed, E>) -> LocalVerdict {
    match parsed {
        Ok(parsed) => accepted(&parsed.statements, !parsed.gaps.is_empty()),
        Err(_) => LocalVerdict::Rejected,
    }
}

/// SQLQuack's strict PostgreSQL verdict.
pub(crate) fn postgres_verdict(sql: &str) -> LocalVerdict {
    native_verdict(sqlquack_syntax_postgres::parse_strict(sql))
}

/// SQLQuack's strict BigQuery GoogleSQL verdict.
fn bigquery_verdict(sql: &str) -> LocalVerdict {
    native_verdict(sqlquack_syntax_bigquery::parse_strict(sql))
}

/// SQLQuack's strict DuckDB verdict.
fn duckdb_verdict(sql: &str) -> LocalVerdict {
    native_verdict(sqlquack_syntax_duckdb::parse_strict(sql))
}

/// The verdict of each dialect's local parser, or none when the dialect has no parser yet.
pub(crate) fn local_verdict(parser: LocalParser) -> Option<fn(&str) -> LocalVerdict> {
    match parser {
        LocalParser::Snowflake => Some(snowflake_verdict),
        LocalParser::Postgres => Some(postgres_verdict),
        LocalParser::BigQuery => Some(bigquery_verdict),
        LocalParser::DuckDb => Some(duckdb_verdict),
        LocalParser::None => None,
    }
}

/// Runs git; its exit code, and its trimmed stdout, or why it could not run.
pub(crate) fn git(arguments: &[&str]) -> Result<(i32, String), String> {
    match Command::new("git").args(arguments).output() {
        Ok(output) => Ok((
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        )),
        Err(error) => Err(format!("git {}: {error}", arguments.join(" "))),
    }
}

/// Git's stdout, or a problem naming the failed command.
fn git_output(arguments: &[&str]) -> Result<String, String> {
    match git(arguments)? {
        (0, stdout) => Ok(stdout),
        (code, _) => Err(format!("git {} exited {code}", arguments.join(" "))),
    }
}

/// The parser path's committed tree, none if uncommitted or absent outside Snowflake, or a failure.
pub(crate) fn parser_tree() -> Result<Option<String>, String> {
    let dialect = current_dialect();
    let parser_path: &str = &dialect.parser_path;
    let status: String = git_output(&["status", "--porcelain", "--", parser_path])?;
    let head_path: String = format!("HEAD:{parser_path}");
    if dialect.name != SNOWFLAKE_DIALECT && git(&["cat-file", "-e", &head_path])?.0 != 0 {
        return Ok(None);
    }
    let tree: String = git_output(&["rev-parse", &head_path])?;
    Ok(status.is_empty().then_some(tree))
}

/// The full id of `commit`, when the catalogue in the working tree under `root` equals its own.
pub(crate) fn frozen_catalogue(commit: &str, root: &str) -> Result<String, String> {
    let full: String = git_output(&["rev-parse", "--verify", &format!("{commit}^{{commit}}")])?;
    let paths: [String; 3] = [
        format!("{root}/catalogue"),
        format!("{root}/sources.toml"),
        format!("{root}/aliases.toml"),
    ];
    let mut arguments: Vec<&str> = vec!["diff", "--quiet", &full, "--"];
    arguments.extend(paths.iter().map(String::as_str));
    match git(&arguments)? {
        (0, _) => Ok(full),
        (1, _) => Err(format!(
            "the catalogue under {root} differs from commit {full}"
        )),
        (code, _) => Err(format!("git diff exited {code}")),
    }
}
