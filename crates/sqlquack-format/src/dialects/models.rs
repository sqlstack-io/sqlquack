//! What a dialect's profile says: how its SQL is read, and the layout rules only it has.

use sqlquack_syntax::Dialect;
use sqlquack_syntax::tokens::models::TokenType;

use crate::dialects::types::{KeywordTyper, NativeLexer, NativeParser};

/// How one dialect's SQL is tokenized, parsed and laid out.
#[derive(Debug)]
pub(crate) struct FormatProfile {
    /// The dialect the shared tokenizer and parser read the SQL in, where the dialect's own scanner or parser does not.
    pub(crate) dialect: Dialect,
    /// The dialect's own scanner, which reads the SQL in place of the shared tokenizer.
    pub(crate) lex: Option<NativeLexer>,
    /// Gives the shared tokens the types the dialect's own scanner reads them as.
    pub(crate) keywords: Option<KeywordTyper>,
    /// The dialect's own parser, in place of the shared one, with where it read each statement when it reports that.
    pub(crate) parse: Option<NativeParser>,
    /// The layout rules only the dialect has.
    pub(crate) rules: LayoutRules,
}

/// Where a parser read one statement, nested ones included, as bytes of the SQL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StatementSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
    /// How many statements enclose it.
    pub(crate) depth: usize,
}

/// The layout rules a dialect has beyond the shared ones; the default turns every one off.
#[derive(Debug)]
pub(crate) struct LayoutRules {
    /// Calls the guide keeps on one line however wide.
    pub(crate) inline_calls: &'static [&'static str],
    /// Characters besides the shared operator characters that the dialect lexes as part of an operator.
    pub(crate) operator_bytes: &'static str,
    /// Whether a word is reserved in the dialect, for names before `(` that keep their case.
    pub(crate) is_reserved: fn(&str) -> bool,
    /// Bind variables such as `:1`, and string stage references with options, are refused.
    pub(crate) refuses_binds: bool,
    /// Select items without an alias are kept as written, so every select list must be laid out.
    pub(crate) keeps_unaliased_items: bool,
    /// Each AND and OR of a condition starts its own line.
    pub(crate) breaks_conditions: bool,
    /// A `:` between a value and a word is a path accessor such as `v:field`.
    pub(crate) path_colons: bool,
    /// A `:` before `[` is a path accessor too, such as `v:['field']`.
    pub(crate) bracket_paths: bool,
    /// A stage reference such as `@stage/path/file.csv` is printed touching as written.
    pub(crate) stage_runs: bool,
    /// A dashed name such as `my-project.ds.t` is printed touching as written.
    pub(crate) dashed_names: bool,
    /// The table of a REFERENCES or of an index's ON keeps its case before `(`.
    pub(crate) referenced_table_names: bool,
    /// Words before a set operator that start its line with it.
    pub(crate) set_operator_prefixes: &'static [&'static str],
    /// A `:` binds tightly outside braces, except after a lambda's parameters.
    pub(crate) tight_colons: bool,
    /// A `%` after a number with no operand after it is a percentage suffix, as in `LIMIT 10%`.
    pub(crate) percent_suffix: bool,
    /// A query may start with FROM.
    pub(crate) from_first: bool,
    /// The quantifiers of a quantified pattern match such as `x LIKE ANY (...)`.
    pub(crate) quantifiers: &'static [&'static str],
    /// The quantifier spaced from its `(` after a pattern operator.
    pub(crate) spaced_quantifier: &'static str,
    /// Clauses after a CTE body, each with the word that ends it, each on its own line.
    pub(crate) cte_clauses: &'static [(&'static str, &'static str)],
    /// Words that start a clause wherever they stand.
    pub(crate) clause_keywords: &'static [&'static str],
    /// Words that with `BY` start a clause listing items as GROUP BY does, such as `CLUSTER BY`.
    pub(crate) by_clauses: &'static [&'static str],
    /// Words that after `FOR` start a clause listing items as GROUP BY does, such as SQL Server's `FOR XML`.
    pub(crate) for_clauses: &'static [&'static str],
    /// Table sources whose call stays on one line wherever it stands.
    pub(crate) inline_sources: &'static [&'static str],
    /// A `TOP (SELECT ...)` count is laid out as a subquery.
    pub(crate) top_queries: bool,
    /// An unreserved word before `(` after `object_name_preceders` or a FROM list comma names an object and keeps its case.
    pub(crate) object_names: bool,
    /// Keywords after which a word names a table, view, routine or index target.
    pub(crate) object_name_preceders: &'static [&'static str],
    /// The commas of a FROM list are told apart from other commas.
    pub(crate) from_list_commas: bool,
    /// The clauses holding comma lists, whose last one at each depth decides whether a comma is a FROM list's.
    pub(crate) list_clauses: &'static [&'static str],
    /// Words that, before APPLY, make an apply join.
    pub(crate) apply_prefixes: &'static [&'static str],
    /// Keywords the guide separates from a following `(`, beyond the shared ones.
    pub(crate) spaced_before_parenthesis: &'static [&'static str],
    /// A parameter token such as `@variable` is whole, so no operator character can extend it.
    pub(crate) whole_variables: bool,
    /// Words after BEGIN that make it a statement, not a block.
    pub(crate) begin_statements: &'static [&'static str],
    /// Statements whose body is a branch or loop.
    pub(crate) branch_statements: &'static [&'static str],
    /// The words of a block that each start their own line.
    pub(crate) block_words: &'static [&'static str],
}

impl Default for LayoutRules {
    fn default() -> Self {
        Self {
            inline_calls: &[],
            operator_bytes: "",
            is_reserved: |_| false,
            refuses_binds: false,
            keeps_unaliased_items: false,
            breaks_conditions: false,
            path_colons: false,
            bracket_paths: false,
            stage_runs: false,
            dashed_names: false,
            referenced_table_names: false,
            set_operator_prefixes: &[],
            tight_colons: false,
            percent_suffix: false,
            from_first: false,
            quantifiers: &[],
            spaced_quantifier: "",
            cte_clauses: &[],
            clause_keywords: &[],
            by_clauses: &[],
            for_clauses: &[],
            inline_sources: &[],
            top_queries: false,
            object_names: false,
            object_name_preceders: &[],
            from_list_commas: false,
            list_clauses: &[],
            apply_prefixes: &[],
            spaced_before_parenthesis: &[],
            whole_variables: false,
            begin_statements: &[],
            branch_statements: &[],
            block_words: &[],
        }
    }
}

/// One significant token with its byte range and source text.
#[derive(Debug, Clone)]
pub(crate) struct Lexeme {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: TokenType,
    pub(crate) raw: String,
}

/// A comment between significant tokens.
#[derive(Debug, Clone)]
pub(crate) struct Comment {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) text: String,
    /// A comment that runs to the end of its line.
    pub(crate) line: bool,
    /// Only whitespace precedes the comment on its line.
    pub(crate) leading: bool,
}

/// How a dialect writes comments, beyond `--` and `/* */`.
#[cfg(feature = "native-scanner")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct CommentSyntax {
    /// `#` starts a comment that runs to the end of the line.
    pub(crate) hash: bool,
    /// Block comments nest.
    pub(crate) nested: bool,
}
