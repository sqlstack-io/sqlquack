//! The PostgreSQL profile: how PostgreSQL resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::dialects::types::DialectProfile;
use crate::lineage::models::Engine;
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
