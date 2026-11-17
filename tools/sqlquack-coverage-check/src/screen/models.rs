//! One-token changes, the allowlist of accepted collapses, and what one screen run found.

use serde::{Deserialize, Serialize};

/// How a variant changes one token of a case's text, or what it writes before one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationKind {
    /// The token is removed.
    Drop,
    /// A keyword is written as one of its synonyms, such as TEMP for TEMPORARY.
    Swap,
    /// A bare or quoted name is written in backquotes.
    Backquote,
    /// A bare, backquoted or double-quoted name is written as a '...' string literal.
    String,
    /// A bare or backquoted name is written in double quotes.
    DoubleQuote,
    /// A '...' string is written as "..." or the reverse: the lexer reads both as one string literal.
    Requote,
    /// A backquoted or quoted name is written bare.
    Unquote,
    /// `AS` is written before the token, or after the last one.
    InsertAs,
    /// `=` is written before the token, or after the last one.
    InsertEquals,
    /// A matching `(` and `)` are both removed, or one token is wrapped in a new pair.
    ParenPair,
}

/// One variant of a case's text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mutation {
    pub kind: MutationKind,
    /// The token as the case wrote it, and its place among the text's tokens.
    pub token: String,
    pub position: usize,
    /// Whether the token is the text's last.
    pub last: bool,
    /// Whether the changed bytes lie wholly inside a comment's body, between its delimiters.
    pub untokenized: bool,
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

/// `screen.toml`: the same-tree variants that are no collapse, each with why.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenAllowlist {
    /// Respellings of one token that the lexer reads as the same token, in every form.
    #[serde(default)]
    pub lexical: Vec<LexicalSpelling>,
    #[serde(default)]
    pub allowed: Vec<AllowedSameTree>,
}

/// A respelling the lexer reads as the same token wherever it is written; only `requote` qualifies.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LexicalSpelling {
    pub mutation: MutationKind,
    pub reason: String,
}

/// One form's same-tree variants of one kind and one token, or inside untokenized text, that are no collapse.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AllowedSameTree {
    pub form: String,
    pub mutation: MutationKind,
    #[serde(default)]
    pub token: Option<String>,
    /// Only when the changed token is the text's last, such as the `;` that ends the statement.
    #[serde(default)]
    pub last_token: bool,
    /// Only changes wholly inside a comment's body; set when `token` is not.
    #[serde(default)]
    pub untokenized: bool,
    pub reason: String,
}

/// A screen run held against the allowlist; it passes only when `flagged`, `stale` and `unpinned` are empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ScreenReport {
    /// Cases the parser accepts, and the variants of them it parsed.
    pub cases: usize,
    pub variants: usize,
    /// Same-tree variants no entry allows.
    pub flagged: Vec<SameTree>,
    /// Same-tree variants an entry allows, with its reason.
    pub allowed: Vec<(SameTree, String)>,
    /// Unused entries: of a screened form, or of any form on a full run.
    pub stale: Vec<String>,
    /// Entries that name neither a token nor untokenized text, and lexical entries of another kind.
    pub unpinned: Vec<String>,
}
