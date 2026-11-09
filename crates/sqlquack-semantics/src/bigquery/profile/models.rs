//! The BigQuery profile: how BigQuery resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::bigquery::profile::_helpers::typing::{
    bigquery_cast_spelling, bigquery_common_type, bigquery_literal_type, bigquery_type,
};
use crate::bigquery::profile::constants::{BIGQUERY_DATE_PART_FUNCTIONS, BIGQUERY_WEEKDAY_PARTS};
use crate::dialects::types::DialectProfile;
use crate::lineage::constants::SNOWFLAKE_VALUES_COLUMN;
use crate::lineage::models::Engine;
use crate::lineage::models::{Lineage, LineageContext, TransformKind};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;

/// BigQuery: columns, aliases, CTEs and relations match whatever their ASCII case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BigQuery;

impl DialectProfile for BigQuery {
    fn name(&self) -> &'static str {
        "BigQuery"
    }

    fn dialect(&self) -> Dialect {
        Dialect::BigQuery
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_ascii_lowercase()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left.eq_ignore_ascii_case(right)
    }
}

/// BigQuery's lineage rules.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BigQueryRules;

impl DialectRules for BigQueryRules {
    fn coercion_engine(&self) -> Engine {
        Engine::BigQuery
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
        false
    }

    fn refuses_distinct_order_by_unselected(&self) -> bool {
        false
    }

    fn date_part_functions(&self) -> &'static [(&'static str, usize)] {
        &BIGQUERY_DATE_PART_FUNCTIONS
    }

    fn weekday_date_parts(&self) -> &'static [&'static str] {
        &BIGQUERY_WEEKDAY_PARTS
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
        true
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

    fn qualifiers_reach_every_name(&self) -> bool {
        false
    }

    fn qualifiers_name_items_by_name(&self) -> bool {
        false
    }

    fn window_only_functions(&self) -> &'static [&'static str] {
        &[]
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
        (bigquery_literal_type(expression).map(Rc::from), generic.1)
    }

    fn computed_type(
        &self,
        expression: &Expression,
        parts: &[(&Expression, Lineage)],
        context: LineageContext<'_>,
        generic: ValueType,
    ) -> ValueType {
        (bigquery_type(expression, parts, context), generic.1)
    }

    fn merged_type(&self, typed: &[&Lineage]) -> Option<Rc<str>> {
        let branches: Vec<(Option<Rc<str>>, bool)> = typed
            .iter()
            .map(|part| (part.data_type.clone(), part.kind == TransformKind::Constant))
            .collect();
        bigquery_common_type(&branches)
    }

    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        bigquery_cast_spelling(data_type)
    }
}
