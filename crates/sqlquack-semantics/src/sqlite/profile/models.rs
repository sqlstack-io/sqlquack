//! The SQLite profile: how SQLite resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::dialects::types::DialectProfile;
use crate::lineage::constants::DATE_PART_FUNCTIONS;
use crate::lineage::models::{CteAliasWidth, Engine, OutputReach, QualifierTargets};
use crate::lineage::models::{Lineage, LineageContext, Nullability};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;
use crate::postgres::profile::main::postgres_window_only::postgres_window_only;
use crate::sqlite::profile::_helpers::typing::{sqlite_cast_spelling, sqlite_literal_type};
use crate::sqlite::profile::constants::SQLITE_VALUES_COLUMN;

/// SQLite: identifiers keep their case as written, and names compare without regard to ASCII case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sqlite;

impl DialectProfile for Sqlite {
    fn name(&self) -> &'static str {
        "SQLite"
    }

    fn dialect(&self) -> Dialect {
        Dialect::SQLite
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_owned()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left.eq_ignore_ascii_case(right)
    }
}

/// SQLite's lineage rules; window-only calls are those PostgreSQL's catalogue lists.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SqliteRules;

impl DialectRules for SqliteRules {
    fn coercion_engine(&self) -> Engine {
        Engine::Sqlite
    }

    fn checks_names_only(&self) -> bool {
        false
    }

    fn output_reach(&self) -> OutputReach {
        OutputReach::Anywhere
    }

    fn checks_call_signatures(&self) -> bool {
        false
    }

    fn checks_set_widths_only(&self) -> bool {
        true
    }

    fn cte_alias_width(&self) -> CteAliasWidth {
        CteAliasWidth::Exact
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
        true
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
        format!("{SQLITE_VALUES_COLUMN}{}", position + 1)
    }

    fn pivots_name_bare(&self) -> bool {
        false
    }

    fn findings_keep_written_case(&self) -> bool {
        false
    }

    fn value_keywords(&self) -> &'static [&'static str] {
        &[]
    }

    fn single_join_prefers_left(&self) -> bool {
        false
    }

    fn messages_name_unstored_items_by_alias(&self) -> bool {
        false
    }

    fn qualifier_targets(&self) -> QualifierTargets {
        QualifierTargets::OpenItems
    }

    fn pseudo_columns(&self) -> &'static [&'static str] {
        &[]
    }

    fn has_connect_by_level(&self) -> bool {
        false
    }

    fn checks_columns_patterns(&self) -> bool {
        false
    }

    fn system_columns(&self) -> &'static [&'static str] {
        &[]
    }

    fn literal_type(&self, expression: &Expression, generic: ValueType) -> ValueType {
        match expression {
            Expression::Literal(literal) => (sqlite_literal_type(literal).map(Rc::from), generic.1),
            _ => (None, generic.1),
        }
    }

    /// No type, since SQLite's dynamic typing is not recorded; nullable only when known to be.
    fn computed_type(
        &self,
        _expression: &Expression,
        _parts: &[(&Expression, Lineage)],
        _context: LineageContext<'_>,
        generic: ValueType,
    ) -> ValueType {
        let nullability: Nullability = match generic.1 {
            Nullability::Nullable => Nullability::Nullable,
            _ => Nullability::Unknown,
        };
        (None, nullability)
    }

    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        sqlite_cast_spelling(data_type)
    }

    /// No name: SQLite leaves an unaliased output that is not a column unnamed here.
    fn unaliased_name(&self, _projection: &Expression) -> Option<String> {
        Some(String::new())
    }

    fn window_only(&self, expression: &Expression) -> bool {
        postgres_window_only(expression)
    }
}
