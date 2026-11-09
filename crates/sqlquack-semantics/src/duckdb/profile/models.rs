//! The DuckDB profile: how DuckDB resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::dialects::types::DialectProfile;
use crate::duckdb::profile::_helpers::duck_types::{
    DuckType, cast_spelling, common_type, parsed_type,
};
use crate::duckdb::profile::_helpers::functions::{duckdb_arities, duckdb_function};
use crate::duckdb::profile::_helpers::typing::{duckdb_literal_type, duckdb_type};
use crate::lineage::models::{Engine, OutputReach};
use crate::lineage::models::{Lineage, LineageContext};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;
use crate::postgres::profile::main::postgres_window_only::postgres_window_only;

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

/// DuckDB's lineage rules; window-only calls are those PostgreSQL's catalogue lists.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DuckDbRules;

impl DialectRules for DuckDbRules {
    fn coercion_engine(&self) -> Engine {
        Engine::DuckDB
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

    fn cast_spelling(&self, data_type: &DataType) -> Option<String> {
        cast_spelling(data_type)
    }

    fn window_only(&self, expression: &Expression) -> bool {
        postgres_window_only(expression)
    }

    fn builtin_arities(&self, name: &str) -> Option<Vec<(usize, Option<usize>)>> {
        duckdb_arities(name)
    }

    fn unknown_function(&self, name: &str) -> bool {
        !duckdb_function(name)
    }
}
