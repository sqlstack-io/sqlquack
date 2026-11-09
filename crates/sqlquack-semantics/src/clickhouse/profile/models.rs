//! The ClickHouse profile: how ClickHouse resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::clickhouse::profile::_helpers::type_names::clickhouse_type as canonical;
use crate::clickhouse::profile::_helpers::typing::{
    clickhouse_common_type, clickhouse_literal_type, clickhouse_type,
};
use crate::clickhouse::profile::constants::CLICKHOUSE_TIMESTAMP_TYPE;
use crate::dialects::types::DialectProfile;
use crate::lineage::main::rendered_type::rendered_type;
use crate::lineage::models::{Engine, OutputReach};
use crate::lineage::models::{Lineage, LineageContext};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;
use crate::postgres::profile::main::postgres_window_only::postgres_window_only;

/// ClickHouse: names keep their case and compare exactly; types compare as ClickHouse names them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClickHouse;

impl DialectProfile for ClickHouse {
    fn name(&self) -> &'static str {
        "ClickHouse"
    }

    fn dialect(&self) -> Dialect {
        Dialect::ClickHouse
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_owned()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left == right
    }

    fn timestamp_type(&self) -> &'static str {
        CLICKHOUSE_TIMESTAMP_TYPE
    }

    fn same_type(&self, left: &str, right: &str) -> bool {
        match (canonical(left), canonical(right)) {
            (Some(left), Some(right)) => left == right,
            _ => left == right,
        }
    }
}

/// ClickHouse's lineage rules; window-only calls are those PostgreSQL's catalogue lists.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ClickHouseRules;

impl DialectRules for ClickHouseRules {
    fn coercion_engine(&self) -> Engine {
        Engine::Snowflake
    }

    fn checks_names_only(&self) -> bool {
        true
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

    fn literal_type(&self, expression: &Expression, generic: ValueType) -> ValueType {
        (clickhouse_literal_type(expression).map(Rc::from), generic.1)
    }

    fn computed_type(
        &self,
        expression: &Expression,
        parts: &[(&Expression, Lineage)],
        context: LineageContext<'_>,
        generic: ValueType,
    ) -> ValueType {
        (clickhouse_type(expression, parts, context), generic.1)
    }

    /// ClickHouse's common type of merged branches, each as ClickHouse names its type.
    fn merged_type(&self, typed: &[&Lineage]) -> Option<Rc<str>> {
        let types: Vec<String> = typed
            .iter()
            .map(|part| part.data_type.as_deref().map(str::to_owned))
            .collect::<Option<_>>()?;
        clickhouse_common_type(&types).map(Rc::from)
    }

    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        rendered_type(data_type).and_then(|written| canonical(&written))
    }

    fn window_only(&self, expression: &Expression) -> bool {
        postgres_window_only(expression)
    }
}
