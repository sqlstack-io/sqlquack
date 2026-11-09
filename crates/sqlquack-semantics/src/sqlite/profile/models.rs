//! The SQLite profile: how SQLite resolves names, and its lineage rules.

use std::rc::Rc;

use sqlquack_syntax::{DataType, Dialect, Expression};

use crate::dialects::types::DialectProfile;
use crate::lineage::models::{Engine, OutputReach};
use crate::lineage::models::{Lineage, LineageContext, Nullability};
use crate::lineage::types::DialectRules;
use crate::lineage::types::ValueType;
use crate::postgres::profile::main::postgres_window_only::postgres_window_only;
use crate::sqlite::profile::_helpers::typing::{sqlite_cast_spelling, sqlite_literal_type};

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

    fn window_only(&self, expression: &Expression) -> bool {
        postgres_window_only(expression)
    }
}
