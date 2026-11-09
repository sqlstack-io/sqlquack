//! The Snowflake profile: how Snowflake resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::dialects::types::DialectProfile;
use crate::lineage::constants::DATE_PART_FUNCTIONS;
use crate::lineage::models::{CteAliasWidth, Engine, OutputReach};
use crate::lineage::models::{Lineage, LineageContext};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;
use crate::postgres::profile::main::postgres_window_only::postgres_window_only;
use crate::snowflake::profile::_helpers::sets::snowflake_set_type;
use crate::snowflake::profile::_helpers::type_names::same_snowflake_type;
use crate::snowflake::profile::_helpers::typing::{
    snowflake_cast_spelling, snowflake_literal_type, snowflake_type, snowflake_unified,
};
use crate::snowflake::profile::constants::SNOWFLAKE_TIMESTAMP_TYPE;

/// Snowflake: unquoted identifiers resolve to upper case, and resolved names compare exactly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snowflake;

impl DialectProfile for Snowflake {
    fn name(&self) -> &'static str {
        "Snowflake"
    }

    fn dialect(&self) -> Dialect {
        Dialect::Snowflake
    }

    fn resolved_identifier(&self, name: &str, quoted: bool) -> String {
        match quoted {
            true => name.to_owned(),
            false => name.to_ascii_uppercase(),
        }
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left == right
    }

    fn same_type(&self, left: &str, right: &str) -> bool {
        same_snowflake_type(left, right)
    }

    fn timestamp_type(&self) -> &'static str {
        SNOWFLAKE_TIMESTAMP_TYPE
    }
}

/// Snowflake with QUOTED_IDENTIFIERS_IGNORE_CASE: quoted identifiers resolve to upper case too.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SnowflakeQuotedIgnoringCase;

impl DialectProfile for SnowflakeQuotedIgnoringCase {
    fn name(&self) -> &'static str {
        "Snowflake"
    }

    fn dialect(&self) -> Dialect {
        Dialect::Snowflake
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_ascii_uppercase()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left == right
    }

    fn same_type(&self, left: &str, right: &str) -> bool {
        same_snowflake_type(left, right)
    }

    fn timestamp_type(&self) -> &'static str {
        SNOWFLAKE_TIMESTAMP_TYPE
    }
}

/// Snowflake's lineage rules; window-only calls are those PostgreSQL's catalogue lists.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SnowflakeRules;

impl DialectRules for SnowflakeRules {
    fn coercion_engine(&self) -> Engine {
        Engine::Snowflake
    }

    fn checks_names_only(&self) -> bool {
        false
    }

    fn output_reach(&self) -> OutputReach {
        OutputReach::Anywhere
    }

    fn checks_call_signatures(&self) -> bool {
        true
    }

    fn checks_set_widths_only(&self) -> bool {
        false
    }

    fn cte_alias_width(&self) -> CteAliasWidth {
        CteAliasWidth::Exact
    }

    fn groups_match_caselessly(&self) -> bool {
        false
    }

    fn refuses_distinct_order_by_unselected(&self) -> bool {
        true
    }

    fn date_part_functions(&self) -> &'static [(&'static str, usize)] {
        &DATE_PART_FUNCTIONS
    }

    fn weekday_date_parts(&self) -> &'static [&'static str] {
        &[]
    }

    fn variants_hold_json_null(&self) -> bool {
        true
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

    fn literal_type(&self, expression: &Expression, generic: ValueType) -> ValueType {
        match expression {
            Expression::Literal(literal) => match snowflake_literal_type(literal) {
                Some(recorded) => (recorded, generic.1),
                None => generic,
            },
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
            snowflake_type(expression, parts).unwrap_or(data_type),
            nullability,
        )
    }

    fn unified_type(&self, operands: &[&Lineage]) -> Option<Rc<str>> {
        snowflake_unified(operands)
    }

    fn set_type(
        &self,
        merged: Option<Rc<str>>,
        branches: &[Lineage],
        _parts: &[Lineage],
    ) -> Option<Rc<str>> {
        match merged {
            None => snowflake_set_type(&branches.iter().collect::<Vec<&Lineage>>()),
            merged => merged,
        }
    }

    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        snowflake_cast_spelling(data_type)
    }

    fn window_only(&self, expression: &Expression) -> bool {
        postgres_window_only(expression)
    }
}
