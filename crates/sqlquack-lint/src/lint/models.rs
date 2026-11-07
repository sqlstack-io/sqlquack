use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sqlquack_syntax::tokens::models::{Span, Token};
use sqlquack_syntax::{Dialect, Expression};

/// SQL text tokenized and parsed under one dialect.
#[derive(Debug)]
pub struct ParsedSql {
    pub dialect: Dialect,
    pub tokens: Vec<Token>,
    pub statements: Vec<Expression>,
}

#[derive(Debug, Default)]
pub(crate) struct QueryFacts {
    pub null_comparisons: Vec<Span>,
    pub implicit_cartesian_joins: Vec<Span>,
    pub joins_without_condition: Vec<Span>,
    pub unordered_limits: Vec<Span>,
    pub redundant_distincts: Vec<Span>,
    pub positional_set_stars: Vec<Span>,
    pub unused_cte_names: Vec<String>,
    pub cross_joins: Vec<Span>,
    pub cte_only_bodies: Vec<Span>,
    pub terminal_selects: Vec<Span>,
    pub nested_ctes: Vec<Span>,
    pub recursive_ctes: Vec<Span>,
    pub invalid_comment_attachments: Vec<Span>,
    pub inline_query_relations: Vec<Span>,
    pub additional: AdditionalQueryFacts,
}

#[derive(Debug, Default)]
pub(crate) struct AdditionalQueryFacts {
    pub bare_unions: Vec<Span>,
    pub duplicate_table_aliases: Vec<Span>,
    pub duplicate_output_aliases: Vec<Span>,
    pub mixed_group_order_references: Vec<Span>,
    pub redundant_else_nulls: Vec<Span>,
    pub parenthesized_distincts: Vec<Span>,
    pub constant_predicates: Vec<Span>,
    pub consecutive_semicolons: Vec<Span>,
    pub redundant_self_aliases: Vec<Span>,
    pub count_one_literals: Vec<Span>,
    pub unstable_row_numbers: Vec<Span>,
    pub null_not_in_predicates: Vec<Span>,
    pub set_arity_mismatches: Vec<Span>,
    pub projected_stars: Vec<Span>,
    pub unaliased_calculations: Vec<Span>,
    pub unused_table_aliases: Vec<Span>,
    pub null_rejected_left_joins: Vec<Span>,
    pub implicit_inner_joins: Vec<Span>,
    pub ambiguous_order_directions: Vec<Span>,
    pub unqualified_multi_source_columns: Vec<Span>,
    pub inconsistent_single_source_qualification: Vec<Span>,
    pub unknown_relation_qualifiers: Vec<Span>,
    pub simple_boolean_cases: Vec<Span>,
    pub nested_else_cases: Vec<Span>,
    pub unused_joined_relations: Vec<Span>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LintRequest {
    pub version: u32,
    pub sql: String,
    pub dialect: String,
    #[serde(default)]
    pub enabled_rules: Option<Vec<String>>,
    #[serde(default)]
    pub ignored_rules: Vec<String>,
    #[serde(default)]
    pub external_identifiers: Vec<String>,
    #[serde(default)]
    pub dependency_identifiers: Vec<String>,
    #[serde(default)]
    pub externally_referenced_ctes: Vec<String>,
    /// The harness the host wraps this body in; absent for an ordinary query.
    #[serde(default)]
    pub fixture_harness: Option<FixtureHarness>,
    #[serde(default)]
    pub allows_dynamic_output_star: bool,
    #[serde(default)]
    pub max_literal_length: Option<usize>,
    #[serde(default)]
    pub max_ranking_order_by: Option<usize>,
    #[serde(default)]
    pub relation_keys: HashMap<String, Vec<Vec<String>>>,
    #[serde(default)]
    pub relation_names: HashMap<String, String>,
    /// The SQL each macro call in the query generated, so column qualification is judged as the macro wrote it.
    #[serde(default)]
    pub sql_macro_outputs: Vec<SqlMacroOutput>,
    /// The configuration file remediation text names; SQLClean's own when absent.
    #[serde(default)]
    pub config_file: Option<String>,
    /// The advice `SQLCLEAN044` gives before its limit setting; SQLClean's own when absent.
    #[serde(default)]
    pub long_literal_guidance: Option<String>,
}

/// The generated wrapper around a test or macro body: its terminal SELECT is ceremonial, and these names are generated.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FixtureHarness {
    /// Prefixes of the CTE names the harness generates, which are reachable even when unread.
    #[serde(default)]
    pub cte_prefixes: Vec<String>,
    /// Prefixes of the CTEs standing in for the body's inputs, which may read the empty relation with `SELECT *`.
    #[serde(default)]
    pub input_cte_prefixes: Vec<String>,
    /// The table function an input CTE reads, as `SELECT * FROM <name>()`, when it has no rows.
    #[serde(default)]
    pub empty_relation: Option<String>,
}

/// The character range one macro call's output fills, and the relations its arguments pass, which a plain parameter slot read expands to qualify with.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct SqlMacroOutput {
    pub start: usize,
    pub end: usize,
    #[serde(default)]
    pub relations: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct AdditionalFactOptions<'a> {
    pub external_identifiers: &'a HashSet<String>,
    pub dependency_identifiers: &'a HashSet<String>,
    pub sql_macro_outputs: &'a [SqlMacroOutput],
    pub allows_ceremonial_select: bool,
    pub allows_dynamic_output_star: bool,
    pub fixture_harness: Option<&'a FixtureHarness>,
}

#[derive(Debug, Serialize)]
pub struct LintResponse {
    pub version: u32,
    pub diagnostics: Vec<LintDiagnostic>,
    /// Why no Rule ran, for a dialect SQLQuack does not parse yet; absent when the SQL was linted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintDiagnostic {
    pub code: &'static str,
    pub message: Cow<'static, str>,
    pub remediation: Cow<'static, str>,
    pub start: usize,
    pub end: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<LintEdit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_unavailable_reason: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LintEdit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Debug, Clone, Copy)]
pub struct LintRuleMetadata {
    pub code: &'static str,
    pub message: &'static str,
    pub remediation: &'static str,
}

/// One lint request's explicit cross joins, with what is known about the relations they read.
#[derive(Debug)]
pub(crate) struct CrossJoinRequest<'a> {
    pub(crate) statements: &'a [Expression],
    pub(crate) tokens: &'a [Token],
    pub(crate) cross_spans: &'a [Span],
    /// Declared keys by dependency sentinel; the empty key proves at most one row.
    pub(crate) relation_keys: &'a HashMap<String, Vec<Vec<String>>>,
    /// The relation name each dependency sentinel stands for.
    pub(crate) relation_names: &'a HashMap<String, String>,
}
