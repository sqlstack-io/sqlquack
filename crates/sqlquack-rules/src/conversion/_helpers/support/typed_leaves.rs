//! Typed VARIANT leaves: DATEs and timestamps Snowflake keeps typed, which DuckDB writes as text.

use std::cell::{Cell, RefCell};

use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;

use sqlquack_duckdb::semi_structured::constants::SOURCE_VARIANT_JSON;

use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Context, Emitted};
use crate::conversion::_helpers::support::tree::outside_conditions;
use crate::conversion::models::ColumnSpec;
use sqlquack_snowflake::typing::models::SnowflakeType;

/// A converted query's SQL and result columns, or its refusal.
pub(crate) type ConvertedQuery = Result<(String, Vec<ColumnSpec>), Refusal>;

/// Typed leaves entering VARIANTs, reads of values not proven free of them, and text comparisons.
#[derive(Debug, Default)]
pub(crate) struct TypedLeaves {
    entered: Cell<bool>,
    decimal: Cell<bool>,
    unproven: Cell<usize>,
    sourced: Cell<usize>,
    compared: RefCell<Option<Construct>>,
    inspected: RefCell<Option<Construct>>,
}

/// How many unproven values, and how many source VARIANTs trusted to hold JSON, were read so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Reads {
    unproven: usize,
    sourced: usize,
}

impl TypedLeaves {
    /// A DATE or timestamp entered a VARIANT, or a column that may hold one was read.
    pub(crate) fn enter(&self) {
        self.entered.set(true);
        self.unproven.set(self.unproven.get() + 1);
    }

    /// A NUMBER with a scale entered a VARIANT, or a column that may hold one was read.
    pub(crate) fn enter_decimal(&self) {
        self.decimal.set(true);
        self.unproven.set(self.unproven.get() + 1);
    }

    /// The type of `value` read at `construct`, recorded when it is a VARIANT not proven free.
    pub(crate) fn inspect(&self, value: &Emitted, construct: &Construct) {
        if value.data_type == SnowflakeType::SemiStructured && !value.untainted {
            self.inspected
                .borrow_mut()
                .get_or_insert_with(|| construct.clone());
        }
    }

    /// A column or alias read: counted unless proven free of typed leaves, and when a source's.
    pub(crate) fn read(&self, (untainted, sourced): (bool, bool)) {
        if !untainted {
            self.unproven.set(self.unproven.get() + 1);
        }
        if sourced {
            self.sourced.set(self.sourced.get() + 1);
        }
    }

    /// The reads so far, to tell what a conversion read since.
    pub(crate) fn reads(&self) -> Reads {
        Reads {
            unproven: self.unproven.get(),
            sourced: self.sourced.get(),
        }
    }

    /// Whether what was converted since `reads` is proven free of typed leaves, and read a source.
    pub(crate) fn since(&self, reads: Reads) -> (bool, bool) {
        let now = self.reads();
        (now.unproven == reads.unproven, now.sourced != reads.sourced)
    }

    /// A VARIANT not proven free of typed leaves compared by its text at `construct`.
    fn compare(&self, construct: &Construct) {
        self.compared
            .borrow_mut()
            .get_or_insert_with(|| construct.clone());
    }

    /// The refusal when both happen in one statement: DuckDB cannot tell a typed leaf from text.
    pub(crate) fn refusal(&self) -> Option<Refusal> {
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
                "TYPEOF of a VARIANT where a VARIANT may hold a whole NUMBER with a scale, which \
                 Snowflake types DECIMAL and DuckDB's JSON holds as an integer"
                    .to_owned(),
            )),
            _ => None,
        }
    }

    /// A converted query, refused when it compares by text where a typed leaf entered.
    pub(crate) fn checked(&self, converted: ConvertedQuery) -> ConvertedQuery {
        let (sql, columns) = converted?;
        match self.refusal() {
            Some(refusal) => Err(refusal),
            None => Ok((sql, self.marked(columns))),
        }
    }

    /// `columns` with each VARIANT not proven free marked as possibly holding typed leaves.
    pub(crate) fn marked(&self, columns: Vec<ColumnSpec>) -> Vec<ColumnSpec> {
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

/// A comparison by text at `construct`, of values proven `(untainted, sourced)`, as recorded.
pub(crate) fn compared_by_text(
    (untainted, sourced): (bool, bool),
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(), Refusal> {
    if !untainted {
        context.typed.compare(construct);
    }
    match sourced {
        true => diverge(context, SOURCE_VARIANT_JSON, construct),
        false => Ok(()),
    }
}

/// A VARIANT `value` told apart from others by its text at `construct`.
pub(crate) fn compared_value(
    value: &Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(), Refusal> {
    match value.data_type == SnowflakeType::SemiStructured {
        true => compared_by_text((value.untainted, value.sourced), construct, context),
        false => Ok(()),
    }
}
