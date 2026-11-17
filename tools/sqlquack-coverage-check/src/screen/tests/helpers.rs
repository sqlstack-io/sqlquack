use serde_json::Value;

use crate::cli::_helpers::local::{local_statements, local_tokens};
use crate::formats::models::LocalParser;
use crate::screen::main::written_tree::written_tree;
use crate::screen::models::{AllowedSameTree, MutationKind, ScreenedCase};

/// The form every screened test case is bound to.
pub(crate) const FORM: &str = "db.ddl.form";

/// The Databricks parser's written tree of `sql`, or none when it refuses the text.
pub(crate) fn databricks_tree(sql: &str) -> Option<Value> {
    local_statements(LocalParser::Databricks)
        .and_then(|parse| parse(sql))
        .as_deref()
        .map(written_tree)
}

/// The Databricks lexer's token byte ranges of `sql`.
pub(crate) fn databricks_tokens(sql: &str) -> Vec<(usize, usize)> {
    local_tokens(LocalParser::Databricks).map_or_else(Vec::new, |ranges| ranges(sql))
}

/// No lexer ranges: the space splitter alone.
pub(crate) fn no_tokens(_sql: &str) -> Vec<(usize, usize)> {
    Vec::new()
}

pub(crate) fn case(sql: &str) -> ScreenedCase {
    ScreenedCase {
        form: FORM.to_owned(),
        case_id: format!("{FORM}.k000000000001"),
        sql: sql.to_owned(),
    }
}

pub(crate) fn entry(form: &str, mutation: MutationKind, token: Option<&str>) -> AllowedSameTree {
    AllowedSameTree {
        form: form.to_owned(),
        mutation,
        token: token.map(str::to_owned),
        last_token: false,
        reason: "comment text".to_owned(),
    }
}
