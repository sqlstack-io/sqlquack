//! Column and value types: Snowflake types with the marks conversion tracks for VARIANTs.

use std::cell::{Cell, RefCell};

use crate::snowflake::typing::models::SnowflakeType;
use crate::snowflake::typing::types::ColumnSpec;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;

/// Typed leaves entering VARIANTs, reads of values not proven free of them, and text comparisons.
#[derive(Debug, Default)]
pub struct TypedLeaves {
    entered: Cell<bool>,
    decimal: Cell<bool>,
    unproven: Cell<usize>,
    sourced: Cell<usize>,
    compared: RefCell<Option<Construct>>,
    inspected: RefCell<Option<Construct>>,
}

/// How many unproven values, and how many source VARIANTs trusted to hold JSON, were read so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reads {
    unproven: usize,
    sourced: usize,
}

impl TypedLeaves {
    /// A DATE or timestamp entered a VARIANT, or a column that may hold one was read.
    pub fn enter(&self) {
        self.entered.set(true);
        self.unproven.set(self.unproven.get() + 1);
    }

    /// A NUMBER with a scale entered a VARIANT, or a column that may hold one was read.
    pub fn enter_decimal(&self) {
        self.decimal.set(true);
        self.unproven.set(self.unproven.get() + 1);
    }

    /// A value of `data_type` read at `construct`, recorded when a VARIANT not proven free.
    pub fn inspect(&self, (data_type, untainted): (SnowflakeType, bool), construct: &Construct) {
        if data_type == SnowflakeType::SemiStructured && !untainted {
            self.inspected
                .borrow_mut()
                .get_or_insert_with(|| construct.clone());
        }
    }

    /// A column or alias read: counted unless proven free of typed leaves, and when a source's.
    pub fn read(&self, (untainted, sourced): (bool, bool)) {
        if !untainted {
            self.unproven.set(self.unproven.get() + 1);
        }
        if sourced {
            self.sourced.set(self.sourced.get() + 1);
        }
    }

    /// The reads so far, to tell what a conversion read since.
    pub fn reads(&self) -> Reads {
        Reads {
            unproven: self.unproven.get(),
            sourced: self.sourced.get(),
        }
    }

    /// Whether what was converted since `reads` is proven free of typed leaves, and read a source.
    pub fn since(&self, reads: Reads) -> (bool, bool) {
        let now = self.reads();
        (now.unproven == reads.unproven, now.sourced != reads.sourced)
    }

    /// A VARIANT not proven free of typed leaves compared by its text at `construct`.
    pub fn compare(&self, construct: &Construct) {
        self.compared
            .borrow_mut()
            .get_or_insert_with(|| construct.clone());
    }

    /// The refusal when both happen in one statement: DuckDB cannot tell a typed leaf from text.
    pub fn refusal(&self) -> Option<Refusal> {
        let compared = self.compared.borrow().clone();
        let inspected = self.inspected.borrow().clone();
        match (self.entered.get(), self.decimal.get(), compared, inspected) {
            (true, _, Some(construct), _) => Some(outside_conditions(
                construct,
                "a VARIANT compared by its text where a VARIANT may hold a DATE or timestamp, \
                 which Snowflake keeps typed and DuckDB writes as text"
                    .to_owned(),
            )),
            (_, true, _, Some(construct)) => Some(outside_conditions(
                construct,
                "TYPEOF, IS_DECIMAL or IS_INTEGER of a VARIANT where a VARIANT may hold a whole NUMBER with a scale, which \
                 Snowflake types DECIMAL and DuckDB's JSON holds as an integer"
                    .to_owned(),
            )),
            _ => None,
        }
    }

    /// `columns` with each VARIANT not proven free marked as possibly holding typed leaves.
    pub fn marked(&self, columns: Vec<ColumnSpec>) -> Vec<ColumnSpec> {
        columns
            .into_iter()
            .map(|column| ColumnSpec {
                typed_leaves: column.typed_leaves
                    || (self.entered.get()
                        && !column.untainted
                        && column.data_type == SnowflakeType::SemiStructured),
                decimal_leaves: column.decimal_leaves
                    || (self.decimal.get()
                        && !column.untainted
                        && column.data_type == SnowflakeType::SemiStructured),
                ..column
            })
            .collect()
    }
}

/// The tables one source file declares, and a warning for each source it leaves out.
#[derive(Debug, Default)]
pub struct SourceTables {
    /// Table names, folded as Snowflake folds them, and their columns as a schema writes them.
    pub tables: Vec<(String, serde_json::Value)>,
    /// One message per source left out, naming the source and the column without a type.
    pub warnings: Vec<String>,
}
