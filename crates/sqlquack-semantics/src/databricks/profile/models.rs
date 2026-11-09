//! The Databricks SQL profile: how Databricks SQL resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Expression};

use crate::databricks::profile::_helpers::type_names::same_databricks_type;
use crate::databricks::profile::_helpers::typing::{
    databricks_cast_spelling, databricks_literal_type, databricks_type,
};
use crate::dialects::types::DialectProfile;
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
}
