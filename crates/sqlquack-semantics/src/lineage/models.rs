//! Column lineage of a query: each output column, the source columns it reads, and how.

use std::collections::BTreeSet;

use crate::dialects::types::DialectProfile;
use crate::lineage::types::{LineageSchema, SharedColumns};

/// How an output column is derived from its source columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransformKind {
    /// It reads no source column.
    Constant,
    /// It is one source column unchanged.
    Direct,
    /// It is a cast of what it reads.
    Cast,
    /// It computes a value from what it reads.
    Expression,
    /// It aggregates what it reads, as a group or window aggregate.
    Aggregation,
}

/// A stored relation's column: its name, and whether that name is quoted (matched exactly).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SchemaColumn {
    /// The name, without quotes.
    pub name: String,
    /// Whether the name is a quoted identifier.
    pub quoted: bool,
}

/// One column of a source relation an output reads.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceColumn {
    /// The relation as named in the query, its parts joined by dots.
    pub relation: String,
    /// The column, spelled as the schema spells it, else as the query writes it.
    pub column: String,
    /// Whether that spelling is a quoted identifier.
    pub quoted: bool,
}

/// The lineage of one output column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputLineage {
    /// The output column's name, as written.
    pub name: String,
    /// The name as it resolves: what a reader of the stored column matches.
    pub key: String,
    /// How it is derived.
    pub kind: TransformKind,
    /// The columns its value is computed from, sorted; columns deciding rows are `row_sources`.
    pub sources: Vec<SourceColumn>,
    /// Whether every column name it reads resolved to one relation.
    pub resolved: bool,
}

/// The lineage of every output column of one query.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryLineage {
    /// The output columns, in order.
    pub outputs: Vec<OutputLineage>,
    /// Whether a star over a relation of unknown columns left outputs out.
    pub open_star: bool,
    /// Columns deciding rows (filters, join keys, groups, CONNECT BY, PIVOT FOR), sorted.
    pub row_sources: Vec<SourceColumn>,
    /// Whether every column a row-deciding expression reads was traced into `row_sources`.
    pub rows_resolved: bool,
}

/// What lineage needs besides the statement: name rules and the known relations.
#[derive(Clone, Copy, Debug)]
pub struct LineageContext<'a> {
    /// How names resolve and compare.
    pub profile: &'a dyn DialectProfile,
    /// The columns of the relations the query reads.
    pub schema: &'a dyn LineageSchema,
    /// The SQL the statement was parsed from with node ranges, to name unaliased outputs.
    pub sql: Option<&'a str>,
}

/// The lineage of one value while it is being resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Lineage {
    pub(crate) sources: SharedColumns,
    pub(crate) kind: TransformKind,
    pub(crate) resolved: bool,
    /// Columns deciding the rows of a query read inside the value, as a subquery's filters.
    pub(crate) rows: SharedColumns,
    /// Whether a column the value reads could not be traced.
    pub(crate) lost: bool,
    /// Whether a column deciding the rows of a query inside the value could not be traced.
    pub(crate) rows_lost: bool,
}

/// One column a FROM item offers, with its lineage.
#[derive(Clone, Debug)]
pub(crate) struct ScopedColumn {
    pub(crate) name: String,
    pub(crate) key: String,
    pub(crate) lineage: Lineage,
}

/// A FROM item: its key, its known columns, and whether it offers others of unknown name.
#[derive(Clone, Debug)]
pub(crate) struct ScopeSource {
    pub(crate) key: Option<String>,
    pub(crate) query: ResolvedQuery,
}

/// A named CTE's outputs.
#[derive(Clone, Debug)]
pub(crate) struct NamedQuery {
    pub(crate) key: String,
    pub(crate) query: ResolvedQuery,
}

/// What one query level sees: its FROM items, the CTEs in reach and the enclosing level.
#[derive(Debug)]
pub(crate) struct LineageScope<'a> {
    pub(crate) sources: Vec<ScopeSource>,
    pub(crate) ctes: &'a [NamedQuery],
    pub(crate) outer: Option<&'a LineageScope<'a>>,
    pub(crate) aliases: Vec<ScopedColumn>,
    pub(crate) lambda_names: Vec<String>,
    /// The keys USING and NATURAL joins merged, read unqualified and once by a bare star.
    pub(crate) merged: Vec<ScopedColumn>,
    /// The named windows of the level, with what their partitions and orders read.
    pub(crate) windows: Vec<(String, Lineage)>,
}

/// A query's outputs while resolving; `open` when it also offers columns not known by name.
#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedQuery {
    pub(crate) columns: Vec<ScopedColumn>,
    pub(crate) open: bool,
    /// The one relation unknown columns come from, when there is exactly one.
    pub(crate) open_relation: Option<String>,
    /// What every unknown column also reads, as a table function's arguments.
    pub(crate) open_reads: Option<Lineage>,
    /// Columns deciding which rows the query returns.
    pub(crate) rows: RowReads,
}

/// The FROM items of a query level: their sources, merged join keys and row dependencies.
#[derive(Clone, Debug, Default)]
pub(crate) struct FromItems {
    pub(crate) sources: Vec<ScopeSource>,
    pub(crate) merged: Vec<ScopedColumn>,
    pub(crate) rows: RowReads,
}

/// The columns that decide a query's rows, and whether any such column could not be traced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RowReads {
    pub(crate) columns: BTreeSet<SourceColumn>,
    pub(crate) lost: bool,
}
