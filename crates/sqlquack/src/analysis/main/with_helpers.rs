//! `with_helpers`: statements run before converted SQL, after the helper macros they need.

use sqlquack_duckdb_blocks::macros::main::helper_statements::helper_statements;

/// The helper macros that `sql` or `statements` call, then `statements`.
pub fn with_helpers(statements: Vec<String>, sql: &str) -> Vec<String> {
    let mut texts: Vec<&str> = statements.iter().map(String::as_str).collect();
    texts.push(sql);
    let mut all = helper_statements(&texts);
    all.extend(statements);
    all
}
