//! Column lineage of a query: each output column, the source columns it reads, and how.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use sqlquack_syntax::expressions::names::models::Column;
use sqlquack_syntax::tokens::models::Span;

use crate::dialects::types::DialectProfile;
use crate::lineage::types::{DialectRules, LineageSchema, SharedColumns};

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
    /// The column's type as the catalog spells it, when known.
    pub data_type: Option<String>,
    /// Whether the column may hold NULL.
    pub nullability: Nullability,
}

/// How a function's nullability follows from its arguments'.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NullabilityRule {
    /// As its first argument.
    FirstArgument,
    /// As its two results, IFF-style: the second and third arguments.
    ConditionalResult,
    /// NULL when its first argument may be; it may also be NULL otherwise.
    SafeCast,
}

/// Whether a value may be NULL.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Nullability {
    /// Not known.
    #[default]
    Unknown,
    /// It may be NULL.
    Nullable,
    /// It is never NULL.
    NonNull,
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
    /// Its type, spelled as the catalog or the cast that decides it spells it, when known.
    pub data_type: Option<String>,
    /// Whether it may be NULL.
    pub nullability: Nullability,
}

/// The lineage of every output column of one query.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryLineage {
    /// The output columns, in order.
    pub outputs: Vec<OutputLineage>,
    /// Whether a star over a relation of unknown columns left outputs out.
    pub open_star: bool,
    /// Whether the outermost query level (a set operation's first branch) projects a star.
    pub projects_star: bool,
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
    /// The value's type, when known.
    pub(crate) data_type: Option<Rc<str>>,
    /// Whether the value may be NULL.
    pub(crate) nullability: Nullability,
    /// Whether the value is a VARIANT that cannot hold a JSON null, as TO_VARIANT of a non-VARIANT.
    pub(crate) json_null_free: bool,
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
    /// Columns the level's WHERE requires to be NOT NULL, never NULL where the level reads them.
    pub(crate) non_null: Vec<&'a Column>,
    /// Each expression's lineage in this level, by node address; any change to the level clears it.
    pub(crate) traced: RefCell<HashMap<usize, Lineage>>,
}

/// A query's outputs while resolving; `open` when it also offers columns not known by name.
#[derive(Clone, Debug, Default)]
pub(crate) struct ResolvedQuery {
    pub(crate) columns: Vec<ScopedColumn>,
    pub(crate) open: bool,
    /// The one relation unknown columns come from, when there is exactly one.
    pub(crate) open_relation: Option<String>,
    /// The stored relation the query is, when the schema knows its columns.
    pub(crate) stored_relation: Option<String>,
    /// Whether the query is a VALUES list, whose generated column names validation does not judge.
    pub(crate) from_values: bool,
    /// The relation a FROM item names, as written in lower case, when it is a table reference.
    pub(crate) written_name: Option<String>,
    /// The table function an unaliased FROM item calls: the one qualifier that may name it.
    pub(crate) function_name: Option<String>,
    /// What every unknown column also reads, as a table function's arguments.
    pub(crate) open_reads: Option<Lineage>,
    /// Columns deciding which rows the query returns.
    pub(crate) rows: RowReads,
    /// Whether an outer join may fill its rows with NULLs, so every column it offers may be NULL.
    pub(crate) outer_nullable: bool,
    /// Whether the FROM item is a derived table, which no qualifier names unless it is aliased.
    pub(crate) derived: bool,
    /// Whether its definition or a CTE it reads has a finding, so names read from it are not.
    pub(crate) faulted: bool,
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

/// A finding of the checks, worded as polyglot words it: a name, type or scope rule broken.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NameFinding {
    /// The polyglot-compatible code: E201 unknown column, E221 ambiguous, E222 unknown qualifier.
    pub code: &'static str,
    /// The message, in polyglot's words.
    pub message: String,
    /// Where the reference is written, when the parse located it.
    pub span: Option<Span>,
    /// The name as written, which the message spells in lower case as polyglot does.
    pub written: String,
    /// When polyglot reports it: 0 scope rules, 1 names, 2 types.
    pub phase: u8,
    /// The node the finding is about, as an address: findings of a phase follow tree order.
    pub(crate) node: usize,
}

/// The shared rules alone, for a profile whose dialect adds none.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SharedRules;

impl DialectRules for SharedRules {
    fn coercion_engine(&self) -> Engine {
        Engine::Snowflake
    }
}

/// The engine whose binder the checks follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Engine {
    DuckDB,
    /// Snowflake; implicit conversion on USING keys isn't checked, as polyglot leaves it.
    Snowflake,
    /// PostgreSQL: no implicit predicate, comparison or string set-operation coercions.
    Postgres,
    /// SQLite, whose dynamic typing is not recorded: no type findings, only scope and name rules.
    Sqlite,
    /// BigQuery (GoogleSQL): as PostgreSQL, and a string literal converts only to a date or time.
    BigQuery,
    /// Databricks SQL, coercions not recorded: no type findings, only scope and name rules.
    Databricks,
}

/// A type's family, the class the type checks compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Family {
    Unknown,
    Boolean,
    Integer,
    Numeric,
    String,
    Binary,
    Date,
    Time,
    Timestamp,
    Interval,
    Json,
    Uuid,
    Array,
    Map,
    Struct,
}

impl Family {
    pub(crate) fn is_numeric(self) -> bool {
        matches!(self, Self::Integer | Self::Numeric)
    }

    /// BigQuery's JSON, ARRAY and STRUCT, which no other type converts to or from implicitly.
    pub(crate) fn converts_to_nothing(self) -> bool {
        matches!(self, Self::Json | Self::Array | Self::Struct)
    }

    pub(crate) fn is_temporal(self) -> bool {
        matches!(
            self,
            Self::Date | Self::Time | Self::Timestamp | Self::Interval
        )
    }

    /// Whether the checks model the family's conversions fully.
    pub(crate) fn fully_modelled(self) -> bool {
        !matches!(
            self,
            Self::Unknown | Self::Json | Self::Uuid | Self::Array | Self::Map | Self::Struct
        )
    }

    /// The family's name in a finding's words.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Numeric => "numeric",
            Self::String => "string",
            Self::Binary => "binary",
            Self::Date => "date",
            Self::Time => "time",
            Self::Timestamp => "timestamp",
            Self::Interval => "interval",
            Self::Json => "json",
            Self::Uuid => "uuid",
            Self::Array => "array",
            Self::Map => "map",
            Self::Struct => "struct",
        }
    }
}
