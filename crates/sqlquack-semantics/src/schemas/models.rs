//! Columns, relations and project functions over a dialect's own types.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::schemas::types::DialectType;

/// One column: its name, type and proven facts; the fields from `holding` on record how the conversion target holds its values, which only the source dialect's src/<dialect>/ code reads (fensu XSQ009).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ColumnSpec<T: DialectType> {
    /// The name as the dialect resolves it.
    pub name: String,
    /// The dialect's type.
    pub data_type: T,
    /// Whether the column is proven to hold no NULL, as VALUES rows of non-NULL literals are.
    #[serde(skip)]
    pub not_null: bool,
    /// Whether the column depends on an array order the dialect leaves undefined.
    #[serde(skip)]
    pub unordered: bool,
    /// How a semi-structured column's values are held in the conversion target.
    #[serde(skip_serializing_if = "Holding::is_json")]
    pub holding: Holding,
    /// Whether a semi-structured column may hold a DATE or timestamp, which the source keeps typed.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub typed_leaves: bool,
    /// Whether a semi-structured column may hold a whole number with a scale, which the source types DECIMAL.
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
    /// Whether the column is held at a narrower NUMBER than its type, so reads cast it up.
    #[serde(skip)]
    pub widened: bool,
    /// Whether the column holds SYSTEM$TYPEOF text whose plan-dependent storage suffix is still on.
    #[serde(skip)]
    pub suffixed: bool,
}

impl<T: DialectType> ColumnSpec<T> {
    /// A column held as the conversion target stores its type: semi-structured values as JSON.
    pub fn new(name: impl Into<String>, data_type: T) -> Self {
        Self {
            name: name.into(),
            data_type,
            not_null: false,
            unordered: false,
            holding: Holding::Json,
            typed_leaves: false,
            decimal_leaves: false,
            untainted: false,
            sourced: false,
            utc_offset: false,
            declared_offsets: Vec::new(),
            widened: false,
            suffixed: false,
        }
    }
}

/// How a semi-structured value is held in the conversion target.
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Schema<T: DialectType> {
    /// Relation name to columns, in order.
    pub relations: BTreeMap<String, Vec<ColumnSpec<T>>>,
    /// Project function name, as statements call it once resolved, to its signature.
    pub functions: BTreeMap<String, FunctionSpec<T>>,
}

impl<T: DialectType> Default for Schema<T> {
    fn default() -> Self {
        Self {
            relations: BTreeMap::new(),
            functions: BTreeMap::new(),
        }
    }
}

/// A project function's signature, which a call is typed by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionSpec<T: DialectType> {
    /// The language of its body.
    pub language: FunctionLanguage,
    /// The parameters, in order: a name and a type each.
    pub arguments: Vec<ColumnSpec<T>>,
    /// A scalar type, or the columns of a table function.
    pub returns: FunctionReturns<T>,
    /// How calls behave: volatility, NULL input and batching.
    pub traits: FunctionTraits,
}

/// How a project function's calls behave; absent declarations take the defaults.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionTraits {
    /// Whether calls may be folded and shared.
    #[serde(default)]
    pub volatility: Volatility,
    /// Whether a NULL argument calls the handler.
    #[serde(default)]
    pub null_input: NullInput,
    /// Whether the handler takes batches of rows.
    #[serde(default)]
    pub vectorized: bool,
}

/// A function's volatility: VOLATILE, the default, or IMMUTABLE.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Volatility {
    /// Each call may return another value, so calls are never folded or shared.
    #[default]
    Volatile,
    /// Equal arguments give equal results.
    Immutable,
}

/// What a NULL argument does: CALLED ON NULL INPUT, the default, or RETURNS NULL ON NULL INPUT.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NullInput {
    /// The handler is called with None.
    #[default]
    Called,
    /// The call is NULL without calling the handler.
    ReturnsNull,
}

impl Volatility {
    /// The declaration's name, as schema files and conversion reports write it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Volatile => "volatile",
            Self::Immutable => "immutable",
        }
    }
}

impl NullInput {
    /// The declaration's name, as schema files and conversion reports write it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Called => "called",
            Self::ReturnsNull => "returns_null",
        }
    }
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
pub enum FunctionReturns<T: DialectType> {
    /// One value of this type.
    Scalar(T),
    /// Rows of these columns.
    Table(Vec<ColumnSpec<T>>),
}
