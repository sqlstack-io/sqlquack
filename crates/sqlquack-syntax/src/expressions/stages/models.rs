//! Data models of expressions stages.
use serde::Deserialize;
use serde::Serialize;

use crate::expressions::names::models::Identifier;
use crate::expressions::statements::models::CopyParameter;
use crate::expressions::tree::models::Expression;

/// GET, LIST or REMOVE (Snowflake): a command on the files in a stage.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageCommand {
    /// The command word as written, upper-cased: "GET", "LIST", "LS", "REMOVE" or "RM"
    pub kind: String,
    /// The stage location, a stage reference
    pub stage: Expression,
    /// GET's local directory, such as `file:///tmp/d1/`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Whether GET's local directory was written as a string literal
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub target_quoted: bool,
    /// The options after the locations, such as PARALLEL and PATTERN
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<CopyParameter>,
}

/// A stage's written name part by part: `@[<namespace>.]<stage>`, a table's stage `@[<namespace>.]%<table>`, or the user's stage `@~`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageName {
    /// Each part with its quoting, namespace first; a part left out, as in `@.s1`, is null
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Option<Identifier>>,
    /// A table's stage, whose last part is the table
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub table_stage: bool,
    /// The current user's stage, `@~`
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub user_stage: bool,
}
