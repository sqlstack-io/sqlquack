//! The current parser and the Git tree it was built from.

use std::process::Command;

use sqlquack_core::parsing::constants::MAX_FUNCTION_CALL_DEPTH;
use sqlquack_syntax::{
    CallMode, ComplexityGuardOptions, Dialect, Expression, ExpressionWalk, ParseOptions,
};

use crate::check::models::{LocalFailure, LocalVerdict};
use crate::formats::main::content_sha256::content_sha256;

/// The parser crate, relative to the repository root.
const PARSER_PATH: &str = "crates/sqlquack-syntax";
/// The code prefix of every complexity-guard error, which is never a syntax verdict.
const GUARD_PREFIX: &str = "E_GUARD_";

/// SQLQuack's Snowflake verdict under the product's call-depth guard; a Command or Raw node makes it opaque.
pub(crate) fn snowflake_verdict(sql: &str) -> LocalVerdict {
    match sqlquack_syntax::parse_statements_with(
        sql,
        Dialect::Snowflake,
        &ParseOptions {
            complexity_guard: Some(ComplexityGuardOptions {
                max_function_call_depth: Some(MAX_FUNCTION_CALL_DEPTH),
                ..ComplexityGuardOptions::default()
            }),
        },
        CallMode::Faithful,
    ) {
        Ok(statements) => LocalVerdict::Accepted {
            opaque: statements.iter().any(|statement| {
                statement
                    .contains(|node| matches!(node, Expression::Command(_) | Expression::Raw(_)))
            }),
            tree_sha256: Some(match serde_json::to_string(&statements) {
                Ok(tree) => content_sha256(tree.as_bytes()),
                Err(error) => content_sha256(format!("unserialisable tree: {error}").as_bytes()),
            }),
        },
        Err(sqlquack_syntax::Error::Parse { message, .. }) if message.starts_with(GUARD_PREFIX) => {
            LocalVerdict::Failed {
                failure: LocalFailure::Guard,
            }
        }
        Err(_) => LocalVerdict::Rejected,
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

/// The committed parser tree, none when the parser has uncommitted changes, or why git failed.
pub(crate) fn parser_tree() -> Result<Option<String>, String> {
    let status: String = git_output(&["status", "--porcelain", "--", PARSER_PATH])?;
    let tree: String = git_output(&["rev-parse", &format!("HEAD:{PARSER_PATH}")])?;
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
