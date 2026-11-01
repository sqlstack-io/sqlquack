//! Column and value types: Snowflake types with the marks conversion tracks for VARIANTs.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use serde::Serialize;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_snowflake::typing::models::SnowflakeType;

/// One column of a relation: its Snowflake name, as resolved, and type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ColumnSpec {
    /// The resolved name: upper case for unquoted identifiers, as written for quoted ones.
    pub name: String,
    /// The Snowflake type.
    pub data_type: SnowflakeType,
    /// How a VARIANT column's values are held in DuckDB.
    #[serde(skip_serializing_if = "Holding::is_json")]
    pub holding: Holding,
    /// Whether a VARIANT column may hold a DATE or timestamp, which Snowflake keeps typed.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub typed_leaves: bool,
    /// Whether a VARIANT column may hold a whole NUMBER with a scale, which Snowflake types DECIMAL.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub decimal_leaves: bool,
    /// Whether the column is proven to hold no typed leaf: a base table's, or built from such values.
    #[serde(skip)]
    pub untainted: bool,
    /// Whether the column reads a source or schema VARIANT trusted to hold no typed leaf.
    #[serde(skip)]
    pub sourced: bool,
    /// Whether a TIMESTAMP_TZ column's every value is known to hold the UTC offset, +00:00.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub utc_offset: bool,
    /// The schema columns, as `RELATION.COLUMN`, whose declared offset the UTC mark relies on.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub declared_offsets: Vec<String>,
    /// Whether the column is proven to hold no NULL, as VALUES rows of non-NULL literals are.
    #[serde(skip)]
    pub not_null: bool,
    /// Whether the column depends on an array order Snowflake leaves undefined.
    #[serde(skip)]
    pub unordered: bool,
    /// Whether the column is held at a narrower NUMBER than its type, so reads cast it up.
    #[serde(skip)]
    pub widened: bool,
}

impl ColumnSpec {
    /// A column held as DuckDB stores its Snowflake type: VARIANTs as JSON.
    pub fn new(name: impl Into<String>, data_type: SnowflakeType) -> Self {
        Self {
            name: name.into(),
            data_type,
            holding: Holding::Json,
            typed_leaves: false,
            decimal_leaves: false,
            untainted: false,
            sourced: false,
            utc_offset: false,
            declared_offsets: Vec::new(),
            not_null: false,
            unordered: false,
            widened: false,
        }
    }
}

/// How a semi-structured value is held in DuckDB.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Holding {
    /// DuckDB `JSON`: values parsed from text, base-table columns, and builds without a FLOAT.
    #[default]
    Json,
    /// A DuckDB `VARIANT` built from typed SQL scalars, FLOAT among them, which keeps their types.
    Variant,
    /// A column another model wrote from a VARIANT-held value, read back as JSON: types are lost.
    Chained,
}

impl Holding {
    /// Whether values are held as JSON, which serialised schemas leave unwritten.
    pub fn is_json(&self) -> bool {
        *self == Self::Json
    }
}

/// The relations and project functions a statement may use, keyed by resolved name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Schema {
    /// Relation name to columns, in order.
    pub relations: BTreeMap<String, Vec<ColumnSpec>>,
    /// Project function name, as statements call it once resolved, to its signature.
    pub functions: BTreeMap<String, FunctionSpec>,
}

/// A project function's signature, which a call is typed by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionSpec {
    /// The language of its body.
    pub language: FunctionLanguage,
    /// The parameters, in order: a name and a Snowflake type each.
    pub arguments: Vec<ColumnSpec>,
    /// A scalar type, or the columns of a table function.
    pub returns: FunctionReturns,
}

/// The language a project function is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum FunctionLanguage {
    /// A SQL body, which SQLQuack converts like a model.
    Sql,
    /// A Python handler, called opaquely with the declared types.
    Python,
}

/// What a project function returns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum FunctionReturns {
    /// One value of this type.
    Scalar(SnowflakeType),
    /// Rows of these columns.
    Table(Vec<ColumnSpec>),
}

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
                "TYPEOF of a VARIANT where a VARIANT may hold a whole NUMBER with a scale, which \
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
