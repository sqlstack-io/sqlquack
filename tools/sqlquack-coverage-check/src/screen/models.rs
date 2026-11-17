//! One-token changes, the allowlist of accepted collapses, and what one screen run found.

use serde::{Deserialize, Serialize};

/// How a variant changes one space-separated token of a case's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationKind {
    /// The token is removed.
    Drop,
    /// TEMP and TEMPORARY trade places.
    Swap,
    /// A bare or string-quoted name is written in backquotes.
    Backquote,
    /// A bare or backquoted name is written as a string literal.
    String,
    /// A backquoted or string-quoted name is written bare.
    Unquote,
}

/// One variant of a case's text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mutation {
    pub kind: MutationKind,
    /// The token as the case wrote it, and its place among the text's tokens.
    pub token: String,
    pub position: usize,
    pub sql: String,
}

/// A case to screen, with the form whose obligations it is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenedCase {
    pub form: String,
    pub case_id: String,
    pub sql: String,
}

/// A variant the parser accepts with the same tree as the case it changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SameTree {
    pub form: String,
    pub case_id: String,
    pub mutation: Mutation,
    pub original: String,
}

/// `screen.toml`: every same-tree variant kind a form accepts, with why it is no collapse.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenAllowlist {
    #[serde(default)]
    pub allowed: Vec<AllowedSameTree>,
}

/// One form's same-tree variants of one kind, optionally only of one token, that are no collapse.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AllowedSameTree {
    pub form: String,
    pub mutation: MutationKind,
    #[serde(default)]
    pub token: Option<String>,
    pub reason: String,
}

/// A screen run held against the allowlist; it passes only when `flagged` and `stale` are empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ScreenReport {
    /// Cases the parser accepts, and the variants of them it parsed.
    pub cases: usize,
    pub variants: usize,
    /// Same-tree variants no entry allows.
    pub flagged: Vec<SameTree>,
    /// Same-tree variants an entry allows, with its reason.
    pub allowed: Vec<(SameTree, String)>,
    /// Entries for a screened form that no same-tree variant matches.
    pub stale: Vec<String>,
}
