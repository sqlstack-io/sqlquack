//! The PostgreSQL profile: how PostgreSQL resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::dialects::types::DialectProfile;
use crate::lineage::constants::{DATE_PART_FUNCTIONS, SNOWFLAKE_VALUES_COLUMN};
use crate::lineage::models::{CteAliasWidth, Engine, OutputReach};
use crate::lineage::models::{Lineage, LineageContext};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;
use crate::postgres::profile::_helpers::names::postgres_name;
use crate::postgres::profile::_helpers::typing::{
    postgres_cast_spelling, postgres_literal_type, postgres_set_type, postgres_type,
};
use crate::postgres::profile::main::postgres_window_only::postgres_window_only;

/// PostgreSQL: unquoted identifiers resolve to lower case, and resolved names compare exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Postgres;

impl DialectProfile for Postgres {
    fn name(&self) -> &'static str {
        "PostgreSQL"
    }

    fn dialect(&self) -> Dialect {
        Dialect::PostgreSQL
    }

    fn resolved_identifier(&self, name: &str, quoted: bool) -> String {
        match quoted {
            true => name.to_owned(),
            false => name.to_ascii_lowercase(),
        }
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left == right
    }
}

/// PostgreSQL's lineage rules.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PostgresRules;

impl DialectRules for PostgresRules {
    fn coercion_engine(&self) -> Engine {
        Engine::Postgres
    }

    fn checks_names_only(&self) -> bool {
        false
    }

    fn output_reach(&self) -> OutputReach {
        OutputReach::OutputNamePlaces
    }

    fn checks_call_signatures(&self) -> bool {
        true
    }

    fn checks_set_widths_only(&self) -> bool {
        false
    }

    fn cte_alias_width(&self) -> CteAliasWidth {
        CteAliasWidth::AtMost
    }

    fn groups_match_caselessly(&self) -> bool {
        false
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
        false
    }

    fn self_reading_ctes_see_themselves(&self) -> bool {
        false
    }

    fn recursion_keeps_anchor_types(&self) -> bool {
        false
    }

    fn star_keys_in_place(&self) -> bool {
        false
    }

    fn has_lateral_views(&self) -> bool {
        false
    }

    fn as_of_keeps_unmatched_left(&self) -> bool {
        false
    }

    fn bare_unnest_column(&self) -> Option<&'static str> {
        None
    }

    fn unnests_struct_fields(&self) -> bool {
        false
    }

    fn names_file_scans_by_stem(&self) -> bool {
        false
    }

    fn maps_stored_timestamps(&self) -> bool {
        false
    }

    fn table_function_columns(&self) -> &'static [(&'static str, &'static [&'static str])] {
        &[]
    }

    fn values_column_name(&self, position: usize) -> String {
        format!("{SNOWFLAKE_VALUES_COLUMN}{}", position + 1)
    }

    fn pivots_name_bare(&self) -> bool {
        false
    }

    fn literal_type(&self, expression: &Expression, generic: ValueType) -> ValueType {
        match expression {
            Expression::Literal(literal) => (
                postgres_literal_type(literal).map(Rc::from).or(generic.0),
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
            postgres_type(expression, parts).unwrap_or(data_type),
            nullability,
        )
    }

    fn set_type(
        &self,
        merged: Option<Rc<str>>,
        _branches: &[Lineage],
        parts: &[Lineage],
    ) -> Option<Rc<str>> {
        postgres_set_type(parts).unwrap_or(merged)
    }

    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        postgres_cast_spelling(data_type)
    }

    fn unaliased_name(&self, projection: &Expression) -> Option<String> {
        Some(postgres_name(projection))
    }

    fn window_only(&self, expression: &Expression) -> bool {
        postgres_window_only(expression)
    }
}
