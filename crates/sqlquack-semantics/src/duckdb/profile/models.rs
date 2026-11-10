//! The DuckDB profile: how DuckDB resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::dialects::types::DialectProfile;
use crate::duckdb::profile::_helpers::duck_types::{
    DuckType, cast_spelling, common_type, parsed_type,
};
use crate::duckdb::profile::_helpers::functions::{duckdb_arities, duckdb_function};
use crate::duckdb::profile::_helpers::typing::{duckdb_literal_type, duckdb_type};
use crate::duckdb::profile::constants::{
    DUCKDB_ROWID, DUCKDB_TABLE_FUNCTION_COLUMNS, DUCKDB_UNNEST_COLUMN, DUCKDB_VALUE_KEYWORDS,
    DUCKDB_VALUES_COLUMN,
};
use crate::lineage::constants::{DATE_PART_FUNCTIONS, WINDOW_ONLY_FUNCTIONS};
use crate::lineage::models::Engine;
use crate::lineage::models::{Lineage, LineageContext};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;

/// DuckDB: identifiers keep their case as written, and names compare without regard to ASCII case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DuckDb;

impl DialectProfile for DuckDb {
    fn name(&self) -> &'static str {
        "DuckDB"
    }

    fn dialect(&self) -> Dialect {
        Dialect::DuckDB
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_owned()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left.eq_ignore_ascii_case(right)
    }
}

/// DuckDB's lineage rules; window-only calls are the standard window functions.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DuckDbRules;

impl DialectRules for DuckDbRules {
    fn coercion_engine(&self) -> Engine {
        Engine::DuckDB
    }

    fn checks_names_only(&self) -> bool {
        false
    }

    fn outputs_read_anywhere(&self) -> bool {
        true
    }

    fn group_keys_read_outputs(&self) -> bool {
        false
    }

    fn checks_call_signatures(&self) -> bool {
        true
    }

    fn checks_set_widths_only(&self) -> bool {
        false
    }

    fn refuses_cte_alias_count(&self, _aliases: usize, _outputs: usize) -> bool {
        false
    }

    fn groups_match_caselessly(&self) -> bool {
        true
    }

    fn refuses_distinct_order_by_unselected(&self) -> bool {
        false
    }

    fn date_part_functions(&self) -> &'static [(&'static str, usize)] {
        &DATE_PART_FUNCTIONS
    }

    fn weekday_date_parts(&self) -> &'static [&'static str] {
        &[]
    }

    fn variants_hold_json_null(&self) -> bool {
        false
    }

    fn full_alias_list_ends_values(&self) -> bool {
        true
    }

    fn self_reading_ctes_see_themselves(&self) -> bool {
        true
    }

    fn recursion_keeps_anchor_types(&self) -> bool {
        true
    }

    fn star_keys_in_place(&self) -> bool {
        true
    }

    fn has_lateral_views(&self) -> bool {
        false
    }

    fn as_of_keeps_unmatched_left(&self) -> bool {
        false
    }

    fn bare_unnest_column(&self) -> Option<&'static str> {
        Some(DUCKDB_UNNEST_COLUMN)
    }

    fn unnests_struct_fields(&self) -> bool {
        false
    }

    fn names_file_scans_by_stem(&self) -> bool {
        true
    }

    fn maps_stored_timestamps(&self) -> bool {
        false
    }

    fn table_function_columns(&self) -> &'static [(&'static str, &'static [&'static str])] {
        &DUCKDB_TABLE_FUNCTION_COLUMNS
    }

    fn values_column_name(&self, position: usize) -> String {
        format!("{DUCKDB_VALUES_COLUMN}{position}")
    }

    fn pivots_name_bare(&self) -> bool {
        true
    }

    fn findings_keep_written_case(&self) -> bool {
        false
    }

    fn value_keywords(&self) -> &'static [&'static str] {
        &DUCKDB_VALUE_KEYWORDS
    }

    fn single_join_prefers_left(&self) -> bool {
        false
    }

    fn messages_name_unstored_items_by_alias(&self) -> bool {
        true
    }

    fn qualifiers_reach_every_name(&self) -> bool {
        false
    }

    fn qualifiers_name_items_by_name(&self) -> bool {
        true
    }

    fn window_only_functions(&self) -> &'static [&'static str] {
        &WINDOW_ONLY_FUNCTIONS
    }

    fn pseudo_columns(&self) -> &'static [&'static str] {
        &[DUCKDB_ROWID]
    }

    fn has_connect_by_level(&self) -> bool {
        false
    }

    fn checks_columns_patterns(&self) -> bool {
        true
    }

    fn system_columns(&self) -> &'static [&'static str] {
        &[]
    }

    fn literal_type(&self, expression: &Expression, generic: ValueType) -> ValueType {
        match expression {
            Expression::Literal(literal) => (
                duckdb_literal_type(literal).map(|data_type| Rc::from(data_type.spelling())),
                generic.1,
            ),
            _ => generic,
        }
    }

    fn computed_type(
        &self,
        expression: &Expression,
        parts: &[(&Expression, Lineage)],
        _context: LineageContext<'_>,
        generic: ValueType,
    ) -> ValueType {
        let (data_type, nullability): ValueType = generic;
        (
            duckdb_type(expression, parts).unwrap_or(data_type),
            nullability,
        )
    }

    /// The type DuckDB combines every typed branch into, when each branch's type is modelled.
    fn merged_type(&self, typed: &[&Lineage]) -> Option<Rc<str>> {
        let mut combined: Option<DuckType> = None;
        for part in typed {
            let own: DuckType = parsed_type(part.data_type.as_deref()?)?;
            combined = Some(match combined {
                None => own,
                Some(current) => common_type(&current, &own)?,
            });
        }
        combined.map(|combined| Rc::from(combined.spelling()))
    }

    /// A conversion unless both spellings name the same DuckDB type, such as INT4 and INTEGER.
    fn merge_converts(&self, from: &str, to: &str, profile: &dyn DialectProfile) -> bool {
        match (parsed_type(from), parsed_type(to)) {
            (Some(from), Some(to)) => from != to,
            _ => !profile.same_type(from, to),
        }
    }

    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        cast_spelling(data_type)
    }

    /// A struct field's name, a number's text, and otherwise no name.
    fn unaliased_name(&self, projection: &Expression) -> Option<String> {
        Some(match projection {
            Expression::Dot(dot) => dot.field.name.clone(),
            Expression::Literal(literal) => match literal.as_ref() {
                Literal::Number(text) => text.clone(),
                _ => String::new(),
            },
            _ => String::new(),
        })
    }

    fn builtin_arities(&self, name: &str) -> Option<Vec<(usize, Option<usize>)>> {
        duckdb_arities(name)
    }

    fn unknown_function(&self, name: &str) -> bool {
        !duckdb_function(name)
    }
}
