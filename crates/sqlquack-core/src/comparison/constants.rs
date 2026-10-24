//! The declared spelling-only fields and normalisations, each with a reason and a test.

use crate::comparison::models::{FieldMatch, SpellingNormalisation, SpellingOnlyField};

/// Fields removed from both trees before comparison.
pub const SPELLING_ONLY_FIELDS: &[SpellingOnlyField] = &[
    SpellingOnlyField {
        name: "span",
        matching: FieldMatch::Exact,
        reason: "source positions move with whitespace and layout",
    },
    SpellingOnlyField {
        name: "inferred_type",
        matching: FieldMatch::Exact,
        reason: "type annotation fills it after parsing; trees are compared as parsed",
    },
    SpellingOnlyField {
        name: "comments",
        matching: FieldMatch::Contains,
        reason: "comments carry no meaning",
    },
    SpellingOnlyField {
        name: "double_colon_syntax",
        matching: FieldMatch::Exact,
        reason: "`x::T` and `CAST(x AS T)` are the same cast once parsed",
    },
    SpellingOnlyField {
        name: "alias_explicit_as",
        matching: FieldMatch::Exact,
        reason: "`x AS y` and `x y` name the same alias",
    },
    SpellingOnlyField {
        name: "alias_keyword",
        matching: FieldMatch::Exact,
        reason: "the letter case of the AS keyword",
    },
    SpellingOnlyField {
        name: "use_inner_keyword",
        matching: FieldMatch::Exact,
        reason: "`JOIN` and `INNER JOIN` are the same join",
    },
    SpellingOnlyField {
        name: "use_outer_keyword",
        matching: FieldMatch::Exact,
        reason: "`LEFT JOIN` and `LEFT OUTER JOIN` are the same join",
    },
    SpellingOnlyField {
        name: "original_name",
        matching: FieldMatch::Exact,
        reason: "the node kind fixes the function; this keeps only the alias it was written with",
    },
];

/// Values rewritten to one spelling in both trees before comparison.
pub const SPELLING_NORMALISATIONS: &[SpellingNormalisation] = &[
    SpellingNormalisation {
        description: "unquoted function names (`name` beside `args`) are upper-cased",
        reason: "unquoted function names are case-insensitive, and printers upper-case them",
    },
    SpellingNormalisation {
        description: "dollar-quoted string literals become ordinary string literals",
        reason: "`$$a$$` and `'a'` are the same string value",
    },
    SpellingNormalisation {
        description: "comment wrappers (`annotated`) are replaced by the node they wrap",
        reason: "comments carry no meaning",
    },
];

/// The longest JSON excerpt a difference reports for each side.
pub const DIFFERENCE_EXCERPT_CHARACTERS: usize = 160;
