//! The BigQuery profile: how BigQuery resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Expression};

use crate::bigquery::profile::_helpers::typing::{
    bigquery_cast_spelling, bigquery_common_type, bigquery_literal_type, bigquery_type,
};
use crate::dialects::types::DialectProfile;
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
