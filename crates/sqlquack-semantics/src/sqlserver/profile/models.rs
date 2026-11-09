//! The SQL Server profile: how SQL Server resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::dialects::types::DialectProfile;
use crate::lineage::main::rendered_type::rendered_type;
use crate::lineage::models::{CteAliasWidth, Engine, OutputReach};
use crate::lineage::models::{Lineage, LineageContext};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;
use crate::postgres::profile::main::postgres_window_only::postgres_window_only;
use crate::sqlserver::profile::_helpers::typing::{
    sqlserver_cast_spelling, sqlserver_literal_type, sqlserver_type,
};
use crate::sqlserver::profile::constants::SQLSERVER_TIMESTAMP_TYPE;

/// SQL Server under its default collation: names resolve and compare without regard to case.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SqlServer;

impl DialectProfile for SqlServer {
    fn name(&self) -> &'static str {
        "SQL Server"
    }

    fn dialect(&self) -> Dialect {
        Dialect::SqlServer
    }

    fn resolved_identifier(&self, name: &str, _quoted: bool) -> String {
        name.to_lowercase()
    }

    fn same_identifier(&self, left: &str, right: &str) -> bool {
        left.to_lowercase() == right.to_lowercase()
    }

    fn timestamp_type(&self) -> &'static str {
        SQLSERVER_TIMESTAMP_TYPE
    }
}

/// SQL Server's lineage rules; window-only calls are those PostgreSQL's catalogue lists.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SqlServerRules;

impl DialectRules for SqlServerRules {
    fn coercion_engine(&self) -> Engine {
        Engine::Snowflake
    }

    fn checks_names_only(&self) -> bool {
        false
    }

    fn output_reach(&self) -> OutputReach {
        OutputReach::OrderedNamePlaces
    }

    fn checks_call_signatures(&self) -> bool {
        false
    }

    fn checks_set_widths_only(&self) -> bool {
        true
    }

    fn cte_alias_width(&self) -> CteAliasWidth {
        CteAliasWidth::Unchecked
    }

    fn groups_match_caselessly(&self) -> bool {
        true
    }

    fn refuses_distinct_order_by_unselected(&self) -> bool {
        false
    }

    fn literal_type(&self, expression: &Expression, generic: ValueType) -> ValueType {
        match expression {
            Expression::Literal(literal) => (
                sqlserver_literal_type(literal).map(Rc::from).or(generic.0),
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
            sqlserver_type(expression, parts).unwrap_or(data_type),
            nullability,
        )
    }

    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        sqlserver_cast_spelling(data_type).unwrap_or_else(|| rendered_type(data_type))
    }

    fn window_only(&self, expression: &Expression) -> bool {
        postgres_window_only(expression)
    }
}
