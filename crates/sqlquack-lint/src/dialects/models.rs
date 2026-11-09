//! What a dialect's lint profile says.

use crate::dialects::types::{ReadPredicates, ReadStatements, ReadTokens};

/// How one dialect's SQL is read, and the lint facts only it has.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LintProfile {
    pub(crate) tokens: ReadTokens,
    pub(crate) statements: ReadStatements,
    pub(crate) predicates: ReadPredicates,
    /// The dialect's own parser reads join predicates, so they stop at the last token not a join side, and one that does not parse is skipped.
    pub(crate) own_predicate_parser: bool,
    /// Other dialects' aggregates the dialect has none of, so a call of one has an unknown kind.
    pub(crate) foreign_aggregates: &'static [&'static str],
    /// The parser keeps aggregate calls as plain calls, so they are told apart by name.
    pub(crate) plain_call_aggregates: bool,
    /// The dialect's own aggregates, for a parser that keeps them as plain calls.
    pub(crate) named_aggregates: &'static [&'static str],
    /// Of those, the aggregates that are scalar with more than one argument.
    pub(crate) single_argument_aggregates: &'static [&'static str],
    /// JSON parse functions that take one string argument, in upper case.
    pub(crate) json_parse_functions: &'static [&'static str],
    /// Unquoted names fold to upper case.
    pub(crate) folds_upper: bool,
    /// Quoted names compare without case too.
    pub(crate) quoted_names_ignore_case: bool,
    /// The character that quotes a name.
    pub(crate) identifier_quote: char,
    /// `UNION DISTINCT` is valid, so a bare UNION may be spelled out.
    pub(crate) union_distinct: bool,
}
