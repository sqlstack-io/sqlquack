//! The current parser and the Git tree it was built from.

use std::process::Command;

use sqlquack_syntax::{CallMode, Dialect, Expression, ExpressionWalk, ParseOptions};

use crate::check::models::LocalVerdict;

/// The parser crate, relative to the repository root.
const PARSER_PATH: &str = "crates/sqlquack-syntax";

/// SQLQuack's Snowflake verdict; a Command or Raw node anywhere makes the parse opaque.
pub(crate) fn snowflake_verdict(sql: &str) -> LocalVerdict {
    match sqlquack_syntax::parse_statements_with(
        sql,
        Dialect::Snowflake,
        &ParseOptions::default(),
        CallMode::Faithful,
    ) {
        Ok(statements) => LocalVerdict::Accepted {
            opaque: statements.iter().any(|statement| {
                statement
                    .contains(|node| matches!(node, Expression::Command(_) | Expression::Raw(_)))
            }),
        },
        Err(_) => LocalVerdict::Rejected,
    }
}

fn git(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The committed parser tree, or none when the parser has uncommitted changes.
pub(crate) fn parser_tree() -> Option<String> {
    let status: String = git(&["status", "--porcelain", "--", PARSER_PATH])?;
    let tree: String = git(&["rev-parse", &format!("HEAD:{PARSER_PATH}")])?;
    status.is_empty().then_some(tree)
}
