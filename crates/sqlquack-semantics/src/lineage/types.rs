//! What lineage asks of a schema.

use std::collections::BTreeSet;
use std::rc::Rc;

use sqlquack_syntax::{DataType, Expression};

use crate::lineage::_helpers::checks::window_only::named_window_only;
use crate::lineage::main::rendered_type::rendered_type;
use crate::lineage::models::{Engine, Nullability};
use crate::lineage::models::{
    Lineage, LineageContext, NullabilityRule, SchemaColumn, SourceColumn,
};

/// The known relations: each one's columns in order, by the relation's resolved name.
pub trait LineageSchema: std::fmt::Debug + Sync {
    /// The columns of `relation` (its parts resolved and joined by dots), when known.
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]>;

    /// Known columns of `relation` whose full column list is not known, to type the ones read.
    fn open_columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        let _ = relation;
        None
    }

    /// The type the function `name` returns whatever its arguments, when the catalog knows it.
    fn function_type(&self, name: &str) -> Option<&str> {
        let _ = name;
        None
    }

    /// How the function `name`'s nullability follows from its arguments', when the catalog knows.
    fn function_nullability(&self, name: &str) -> Option<NullabilityRule> {
        let _ = name;
        None
    }

    /// Whether the project declares the function `name`, whose calls the type checks leave alone.
    fn known_function(&self, name: &str) -> bool {
        let _ = name;
        false
    }

    /// The `(least, most)` argument counts the catalog accepts for `name`, when it knows them.
    fn function_arities(&self, name: &str, canonical: &str) -> Option<Vec<(usize, Option<usize>)>> {
        let _ = (name, canonical);
        None
    }

    /// Whether the project declares the type `name`, which a cast may name.
    fn known_type(&self, name: &str) -> bool {
        let _ = name;
        false
    }
}

/// A set of source columns that copies of a value's lineage share until one changes it.
pub(crate) type SharedColumns = Rc<BTreeSet<SourceColumn>>;

/// How one dialect types values, names outputs and knows its functions, beyond the shared rules.
pub(crate) trait DialectRules: Sync {
    /// The engine whose binder the coercion checks follow for this dialect.
    fn coercion_engine(&self) -> Engine;

    /// Whether lineage keeps to the name rules, leaving out the scope, type and set-operation ones.
    fn checks_names_only(&self) -> bool;

    /// Whether every clause of a SELECT, its select list too, may read the outputs named in it.
    fn outputs_read_anywhere(&self) -> bool;

    /// Where outputs are not read anywhere, whether GROUP BY keys may read them as ORDER BY keys do.
    fn group_keys_read_outputs(&self) -> bool;

    /// Whether calls are checked against the signatures of the functions they name.
    fn checks_call_signatures(&self) -> bool;

    /// Whether set operations check only that their branches return as many columns.
    fn checks_set_widths_only(&self) -> bool;

    /// Whether a CTE with `aliases` column aliases over a query of `outputs` columns is refused.
    fn refuses_cte_alias_count(&self, aliases: usize, outputs: usize) -> bool;

    /// Whether GROUP BY expressions match the expressions they group whatever their keyword case.
    fn groups_match_caselessly(&self) -> bool;

    /// Whether a DISTINCT of columns refuses an ORDER BY of a column it does not select.
    fn refuses_distinct_order_by_unselected(&self) -> bool;

    /// The functions that take a date part keyword, each with the part's argument position.
    fn date_part_functions(&self) -> &'static [(&'static str, usize)];

    /// The date parts written with a weekday argument, as `WEEK(MONDAY)`.
    fn weekday_date_parts(&self) -> &'static [&'static str];

    /// Whether a semi-structured value may hold a JSON null that a cast turns into SQL NULL.
    fn variants_hold_json_null(&self) -> bool;

    /// Whether a column alias list naming every output of a VALUES list stops it reading as VALUES.
    fn full_alias_list_ends_values(&self) -> bool;

    /// Whether a CTE whose body reads itself is checked with its own result in reach.
    fn self_reading_ctes_see_themselves(&self) -> bool;

    /// Whether a recursive CTE's columns keep the types its anchor gives them.
    fn recursion_keeps_anchor_types(&self) -> bool;

    /// Whether a star lists each join key once, where and as its first source offers it.
    fn star_keys_in_place(&self) -> bool;

    /// Whether a SELECT may add rows through LATERAL VIEW clauses.
    fn has_lateral_views(&self) -> bool;

    /// Whether an ASOF JOIN keeps unmatched left rows, NULL-filling the right.
    fn as_of_keeps_unmatched_left(&self) -> bool;

    /// The one column an UNNEST in FROM without alias or ordinality returns, when it is a source.
    fn bare_unnest_column(&self) -> Option<&'static str>;

    /// Whether an unaliased UNNEST of an array of STRUCT values returns the structs' fields.
    fn unnests_struct_fields(&self) -> bool;

    /// Whether an unaliased file scan takes its file's stem as its name.
    fn names_file_scans_by_stem(&self) -> bool;

    /// Whether stored TIMESTAMP columns read as the type the adapter creates them as.
    fn maps_stored_timestamps(&self) -> bool;

    /// The dialect's own built-in table functions, each with its columns.
    fn table_function_columns(&self) -> &'static [(&'static str, &'static [&'static str])];

    /// The name of a VALUES list's column at the zero-based `position`.
    fn values_column_name(&self, position: usize) -> String;

    /// Whether pivoted columns take a string's bare text, unnamed beside several aggregates.
    fn pivots_name_bare(&self) -> bool;

    /// Whether a name finding quotes the name as written rather than in lower case.
    fn findings_keep_written_case(&self) -> bool;

    /// Keywords a bare name reads as their value when no column in reach has that name.
    fn value_keywords(&self) -> &'static [&'static str];

    /// Whether a name both sides of a query's one join offer is read from the left side.
    fn single_join_prefers_left(&self) -> bool;

    /// Whether an unknown column's message names a FROM item that is no stored table by its alias.
    fn messages_name_unstored_items_by_alias(&self) -> bool;

    /// Whether a qualifier may name any name in reach: outputs, CTEs, aliases and lambda parameters.
    fn qualifiers_reach_every_name(&self) -> bool;

    /// Whether a qualifier names a FROM item only by its name or its table function's name.
    fn qualifiers_name_items_by_name(&self) -> bool;

    /// The dialect's own pseudo-columns, which no relation lists.
    fn pseudo_columns(&self) -> &'static [&'static str];

    /// Whether a hierarchical query offers the LEVEL pseudo-column.
    fn has_connect_by_level(&self) -> bool;

    /// Whether a COLUMNS pattern that matches no column is a finding.
    fn checks_columns_patterns(&self) -> bool;

    /// The system columns every stored table has and no star expands.
    fn system_columns(&self) -> &'static [&'static str];

    /// The built-in functions the dialect runs only with OVER, by name.
    fn window_only_functions(&self) -> &'static [&'static str];

    /// A literal's type and nullability, given the shared rules' `generic` answer.
    fn literal_type(&self, expression: &Expression, generic: ValueType) -> ValueType {
        let _ = expression;
        generic
    }

    /// A computed value's type and nullability, given the shared rules' `generic` answer.
    fn computed_type(
        &self,
        expression: &Expression,
        parts: &[(&Expression, Lineage)],
        context: LineageContext<'_>,
        generic: ValueType,
    ) -> ValueType {
        let _ = (expression, parts, context);
        generic
    }

    /// The one type operands of differing types unify to, when the dialect unifies them.
    fn unified_type(&self, operands: &[&Lineage]) -> Option<Rc<str>> {
        let _ = operands;
        None
    }

    /// The type alternative branches of differing types combine into, when the dialect knows it.
    fn merged_type(&self, typed: &[&Lineage]) -> Option<Rc<str>> {
        let _ = typed;
        None
    }

    /// A set operation's type, from its `merged` type, its typed `branches` and all its `parts`.
    fn set_type(
        &self,
        merged: Option<Rc<str>>,
        branches: &[Lineage],
        parts: &[Lineage],
    ) -> Option<Rc<str>> {
        let _ = (branches, parts);
        merged
    }

    /// A cast target's spelling, when lineage knows the type.
    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        rendered_type(data_type)
    }

    /// The dialect's name for an unaliased output that is no column; `None` defers to Snowflake's.
    fn unaliased_name(&self, projection: &Expression) -> Option<String> {
        let _ = projection;
        None
    }

    /// Whether `expression` calls a built-in function the dialect runs only with OVER.
    fn window_only(&self, expression: &Expression) -> bool {
        named_window_only(expression, self.window_only_functions())
    }

    /// The `(least, most)` argument counts of the built-in function `name`, when recorded.
    fn builtin_arities(&self, name: &str) -> Option<Vec<(usize, Option<usize>)>> {
        let _ = name;
        None
    }

    /// Whether an unqualified call to `name` names no built-in function the dialect records.
    fn unknown_function(&self, name: &str) -> bool {
        let _ = name;
        false
    }
}

/// A value's type and nullability.
pub(crate) type ValueType = (Option<Rc<str>>, Nullability);
