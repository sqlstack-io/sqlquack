//! The Databricks SQL profile: how Databricks SQL resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::databricks::profile::_helpers::type_names::same_databricks_type;
use crate::databricks::profile::_helpers::typing::{
    databricks_cast_spelling, databricks_literal_type, databricks_type,
};
use crate::dialects::types::DialectProfile;
use crate::lineage::constants::DATE_PART_FUNCTIONS;
use crate::lineage::models::{CteAliasWidth, Engine, OutputReach};
use crate::lineage::models::{Lineage, LineageContext};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;

/// Databricks SQL: names kept as written and compared without ASCII case; types by synonym.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Databricks;

impl DialectProfile for Databricks {
    fn name(&self) -> &'static str {
        "Databricks"
    }

    fn dialect(&self) -> Dialect {
        Dialect::Databricks
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_owned()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left.eq_ignore_ascii_case(right)
    }

    fn same_type(&self, left: &str, right: &str) -> bool {
        same_databricks_type(left, right)
    }
}

/// Databricks SQL's lineage rules.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DatabricksRules;

impl DialectRules for DatabricksRules {
    fn coercion_engine(&self) -> Engine {
        Engine::Databricks
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
        CteAliasWidth::Unchecked
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

    fn literal_type(&self, expression: &Expression, generic: ValueType) -> ValueType {
        match expression {
            Expression::Literal(literal) => {
                (databricks_literal_type(literal).map(Rc::from), generic.1)
            }
            _ => generic,
        }
    }

    fn computed_type(
        &self,
        expression: &Expression,
        parts: &[(&Expression, Lineage)],
        context: LineageContext<'_>,
        generic: ValueType,
    ) -> ValueType {
        let (data_type, nullability): ValueType = generic;
        (
            databricks_type(expression, parts, context).unwrap_or(data_type),
            nullability,
        )
    }

    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        databricks_cast_spelling(data_type)
    }

    /// A struct field's name, and otherwise no name.
    fn unaliased_name(&self, projection: &Expression) -> Option<String> {
        Some(match projection {
            Expression::Dot(dot) => dot.field.name.clone(),
            _ => String::new(),
        })
    }
}
