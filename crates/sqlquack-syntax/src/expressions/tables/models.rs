//! Data models of expressions tables.

use serde::Deserialize;
use serde::Serialize;

use crate::expressions::columns::models::ColumnDef;
use crate::expressions::columns::models::TableConstraint;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::operators::models::PartitionSpec;
use crate::expressions::queries::models::With;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Interval;

/// ON COMMIT behavior for temporary tables
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OnCommit {
    /// ON COMMIT PRESERVE ROWS
    PreserveRows,
    /// ON COMMIT DELETE ROWS
    DeleteRows,
}

/// TiDB `AUTO_RANDOM[(shard_bits[, range_bits])]` column attribute.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TiDBAutoRandom {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shard_bits: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_bits: Option<u64>,
    /// Whether the attribute was wrapped in a TiDB executable comment.
    #[serde(default)]
    pub executable_comment: bool,
}

/// A TiDB-specific table option and its source syntax.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TiDBTableOption {
    pub kind: TiDBTableOptionKind,
    /// Whether the option was wrapped in a TiDB executable comment.
    #[serde(default)]
    pub executable_comment: bool,
}

/// TiDB table options supported by `CREATE TABLE` and applicable `ALTER TABLE` forms.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TiDBTableOptionKind {
    ShardRowIdBits {
        bits: u64,
    },
    PreSplitRegions {
        regions: u64,
    },
    AutoRandomBase {
        value: u64,
    },
    /// `None` represents `PLACEMENT POLICY = DEFAULT`.
    PlacementPolicy {
        policy: Option<Identifier>,
    },
    Ttl {
        column: Identifier,
        interval: Box<Interval>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        enabled: Option<bool>,
    },
    TtlEnable {
        enabled: bool,
    },
    TtlJobInterval {
        interval: String,
    },
}

/// CREATE TABLE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTable {
    pub name: TableRef,
    /// ClickHouse: ON CLUSTER clause for distributed DDL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_cluster: Option<OnCluster>,
    pub columns: Vec<ColumnDef>,
    pub constraints: Vec<TableConstraint>,
    pub if_not_exists: bool,
    pub temporary: bool,
    pub or_replace: bool,
    /// Table modifier: DYNAMIC, ICEBERG, EXTERNAL, HYBRID (Snowflake)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_modifier: Option<String>,
    /// Dialect whose call semantics must be retained through independent generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub source_dialect: Option<crate::tokens::models::DialectType>,

    pub as_select: Option<Expression>,
    /// Whether the AS SELECT was wrapped in parentheses
    #[serde(default)]
    pub as_select_parenthesized: bool,
    /// ON COMMIT behavior for temporary tables
    #[serde(default)]
    pub on_commit: Option<OnCommit>,
    /// Clone source table (e.g., CREATE TABLE t CLONE source_table)
    #[serde(default)]
    pub clone_source: Option<TableRef>,
    /// Time travel AT/BEFORE clause for CLONE (e.g., AT(TIMESTAMP => '...'))
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clone_at_clause: Option<Expression>,
    /// Whether this is a COPY operation (BigQuery) vs CLONE (Snowflake/Databricks)
    #[serde(default)]
    pub is_copy: bool,
    /// Whether this is a SHALLOW CLONE (Databricks/Delta Lake)
    #[serde(default)]
    pub shallow_clone: bool,
    /// Whether this is an explicit DEEP CLONE (Databricks/Delta Lake)
    #[serde(default)]
    pub deep_clone: bool,
    /// Leading comments before the statement
    #[serde(default)]
    pub leading_comments: Vec<String>,
    /// WITH properties (e.g., WITH (FORMAT='parquet'))
    #[serde(default)]
    pub with_properties: Vec<(String, String)>,
    /// Teradata: table options after name before columns (comma-separated)
    #[serde(default)]
    pub teradata_post_name_options: Vec<String>,
    /// Teradata: WITH DATA (true) or WITH NO DATA (false) after AS SELECT
    #[serde(default)]
    pub with_data: Option<bool>,
    /// Teradata: AND STATISTICS (true) or AND NO STATISTICS (false)
    #[serde(default)]
    pub with_statistics: Option<bool>,
    /// Teradata: Index specifications (NO PRIMARY INDEX, UNIQUE PRIMARY INDEX, etc.)
    #[serde(default)]
    pub teradata_indexes: Vec<TeradataIndex>,
    /// WITH clause (CTEs) - for CREATE TABLE ... AS WITH ... SELECT ...
    #[serde(default)]
    pub with_cte: Option<With>,
    /// Table properties like DEFAULT COLLATE (BigQuery)
    #[serde(default)]
    pub properties: Vec<Expression>,
    /// PostgreSQL PARTITION OF property (e.g., CREATE TABLE t PARTITION OF parent ...)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_of: Option<Expression>,
    /// TSQL: WITH(SYSTEM_VERSIONING=ON(...)) after column definitions
    #[serde(default)]
    pub post_table_properties: Vec<Expression>,
    /// MySQL table options after column definitions (ENGINE=val, AUTO_INCREMENT=val, etc.)
    #[serde(default)]
    pub mysql_table_options: Vec<(String, String)>,
    /// TiDB-specific table options after column definitions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tidb_table_options: Vec<TiDBTableOption>,
    /// PostgreSQL INHERITS clause: INHERITS (parent1, parent2, ...)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inherits: Vec<TableRef>,
    /// TSQL ON filegroup or ON filegroup (partition_column) clause
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_property: Option<OnProperty>,
    /// Snowflake: COPY GRANTS clause to copy privileges from replaced table
    #[serde(default)]
    pub copy_grants: bool,
    /// Snowflake: USING TEMPLATE expression for schema inference
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub using_template: Option<Box<Expression>>,
    /// StarRocks: ROLLUP (r1(col1, col2), r2(col1))
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollup: Option<RollupProperty>,
    /// ClickHouse: UUID 'xxx' clause after table name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// WITH PARTITION COLUMNS (name type, ...), as BigQuery external tables use.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with_partition_columns: Vec<ColumnDef>,
    /// WITH CONNECTION `project.region.connection`, as BigQuery external tables use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_connection: Option<TableRef>,
}

/// Teradata index specification for CREATE TABLE
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeradataIndex {
    /// Index kind: NoPrimary, Primary, PrimaryAmp, Unique, UniquePrimary
    pub kind: TeradataIndexKind,
    /// Optional index name
    pub name: Option<String>,
    /// Optional column list
    pub columns: Vec<String>,
}

/// Kind of Teradata index
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TeradataIndexKind {
    /// NO PRIMARY INDEX
    NoPrimary,
    /// PRIMARY INDEX
    Primary,
    /// PRIMARY AMP INDEX
    PrimaryAmp,
    /// UNIQUE INDEX
    Unique,
    /// UNIQUE PRIMARY INDEX
    UniquePrimary,
    /// INDEX (secondary, non-primary)
    Secondary,
}

impl CreateTable {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            on_cluster: None,
            columns: Vec::new(),
            constraints: Vec::new(),
            if_not_exists: false,
            temporary: false,
            or_replace: false,
            table_modifier: None,
            source_dialect: None,
            as_select: None,
            as_select_parenthesized: false,
            on_commit: None,
            clone_source: None,
            clone_at_clause: None,
            shallow_clone: false,
            deep_clone: false,
            is_copy: false,
            leading_comments: Vec::new(),
            with_properties: Vec::new(),
            teradata_post_name_options: Vec::new(),
            with_data: None,
            with_statistics: None,
            teradata_indexes: Vec::new(),
            with_cte: None,
            properties: Vec::new(),
            partition_of: None,
            post_table_properties: Vec::new(),
            mysql_table_options: Vec::new(),
            tidb_table_options: Vec::new(),
            inherits: Vec::new(),
            on_property: None,
            copy_grants: false,
            using_template: None,
            rollup: None,
            uuid: None,
            with_partition_columns: Vec::new(),
            with_connection: None,
        }
    }
}

/// Sort order for PRIMARY KEY ASC/DESC
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Serialize, Deserialize,
)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Type of column constraint for tracking order
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintType {
    NotNull,
    Null,
    PrimaryKey,
    Unique,
    Default,
    AutoIncrement,
    AutoRandom,
    Collate,
    Comment,
    References,
    Check,
    GeneratedAsIdentity,
    /// Snowflake: TAG (key='value', ...)
    Tags,
    /// Computed/generated column
    ComputedColumn,
    /// TSQL temporal: GENERATED ALWAYS AS ROW START|END
    GeneratedAsRow,
    /// MySQL: ON UPDATE expression
    OnUpdate,
    /// PATH constraint for XMLTABLE/JSON_TABLE columns
    Path,
    /// Redshift: ENCODE encoding_type
    Encode,
}

/// DROP TABLE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropTable {
    pub names: Vec<TableRef>,
    pub if_exists: bool,
    pub cascade: bool,
    /// Oracle: CASCADE CONSTRAINTS
    #[serde(default)]
    pub cascade_constraints: bool,
    /// Oracle: PURGE
    #[serde(default)]
    pub purge: bool,
    /// Comments that appear before the DROP keyword (e.g., leading line comments)
    #[serde(default)]
    pub leading_comments: Vec<String>,
    /// TSQL OBJECT_ID arguments, to rebuild an `IF OBJECT_ID(...) IS NOT NULL` drop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_id_args: Option<String>,
    /// ClickHouse: SYNC modifier
    #[serde(default)]
    pub sync: bool,
    /// Snowflake: DROP ICEBERG TABLE
    #[serde(default)]
    pub iceberg: bool,
    /// RESTRICT modifier (opposite of CASCADE)
    #[serde(default)]
    pub restrict: bool,
}

impl DropTable {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            names: vec![TableRef::new(name)],
            if_exists: false,
            cascade: false,
            cascade_constraints: false,
            purge: false,
            leading_comments: Vec::new(),
            object_id_args: None,
            sync: false,
            iceberg: false,
            restrict: false,
        }
    }
}

/// UNDROP object statement (Snowflake, ClickHouse)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Undrop {
    /// The object kind, e.g. "TABLE", "SCHEMA", "DATABASE", "DYNAMIC TABLE"
    pub kind: String,
    /// The object name
    pub name: TableRef,
    /// IF EXISTS clause
    #[serde(default)]
    pub if_exists: bool,
    /// Snowflake: optional RENAME TO target
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_to: Option<TableRef>,
}

/// Partition scope for a TiDB `SPLIT TABLE` statement.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SplitTablePartitionScope {
    Table,
    AllPartitions,
    Partitions { names: Vec<Identifier> },
}

/// Split-point specification for a TiDB `SPLIT TABLE` statement.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SplitTableMode {
    Between {
        lower: Vec<Expression>,
        upper: Vec<Expression>,
        regions: u64,
    },
    By {
        points: Vec<Vec<Expression>>,
    },
}

/// TiDB `SPLIT [PARTITION] TABLE` statement.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitTable {
    pub table: TableRef,
    pub partition_scope: SplitTablePartitionScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<Identifier>,
    pub mode: SplitTableMode,
}

/// TiDB `FLASHBACK TABLE table [TO new_name]` statement.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlashbackTable {
    pub table: TableRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_to: Option<Identifier>,
}

/// CREATE INDEX statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateIndex {
    pub name: Identifier,
    pub table: TableRef,
    pub columns: Vec<IndexColumn>,
    /// Expression key parts, for functional index keys `columns` cannot hold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_parts: Vec<IndexKeyPart>,
    pub unique: bool,
    pub if_not_exists: bool,
    pub using: Option<String>,
    /// TSQL CLUSTERED/NONCLUSTERED modifier
    #[serde(default)]
    pub clustered: Option<String>,
    /// PostgreSQL CONCURRENTLY modifier
    #[serde(default)]
    pub concurrently: bool,
    /// PostgreSQL WHERE clause for partial indexes
    #[serde(default)]
    pub where_clause: Option<Box<Expression>>,
    /// PostgreSQL INCLUDE columns
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include_columns: Vec<Identifier>,
    /// TSQL WITH options (e.g., allow_page_locks=on)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with_options: Vec<(String, String)>,
    /// TSQL ON filegroup or partition scheme (e.g., ON PRIMARY, ON X([y]))
    #[serde(default)]
    pub on_filegroup: Option<String>,
}

impl CreateIndex {
    pub fn new(name: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            table: TableRef::new(table),
            columns: Vec::new(),
            key_parts: Vec::new(),
            unique: false,
            if_not_exists: false,
            using: None,
            clustered: None,
            concurrently: false,
            where_clause: None,
            include_columns: Vec::new(),
            with_options: Vec::new(),
            on_filegroup: None,
        }
    }
}

/// Index column specification
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexColumn {
    pub column: Identifier,
    pub desc: bool,
    /// Explicit ASC keyword was present
    #[serde(default)]
    pub asc: bool,
    pub nulls_first: Option<bool>,
    /// PostgreSQL operator class (e.g., varchar_pattern_ops, public.gin_trgm_ops)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opclass: Option<String>,
}

/// An expression-capable index key part.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexKeyPart {
    pub expression: Box<Expression>,
    /// MySQL column prefix length, for example the `16` in `name(16)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix_length: Option<String>,
    pub desc: bool,
    /// Explicit ASC keyword was present.
    #[serde(default)]
    pub asc: bool,
    pub nulls_first: Option<bool>,
    /// PostgreSQL operator class (e.g., varchar_pattern_ops, public.gin_trgm_ops).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opclass: Option<String>,
}

/// DROP INDEX statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropIndex {
    pub name: TableRef,
    pub table: Option<TableRef>,
    pub if_exists: bool,
    /// PostgreSQL CONCURRENTLY modifier
    #[serde(default)]
    pub concurrently: bool,
}

impl DropIndex {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            table: None,
            if_exists: false,
            concurrently: false,
        }
    }
}

/// Property
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    pub this: Box<Expression>,
    #[serde(default)]
    pub value: Option<Box<Expression>>,
}

/// GrantPrivilege
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrantPrivilege {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// AllowedValuesProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllowedValuesProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// AlgorithmProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlgorithmProperty {
    pub this: Box<Expression>,
}

/// AutoIncrementProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoIncrementProperty {
    pub this: Box<Expression>,
}

/// AutoRefreshProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoRefreshProperty {
    pub this: Box<Expression>,
}

/// BackupProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupProperty {
    pub this: Box<Expression>,
}

/// BuildProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildProperty {
    pub this: Box<Expression>,
}

/// BlockCompressionProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockCompressionProperty {
    #[serde(default)]
    pub autotemp: Option<Box<Expression>>,
    #[serde(default)]
    pub always: Option<Box<Expression>>,
    #[serde(default)]
    pub default: Option<Box<Expression>>,
    #[serde(default)]
    pub manual: Option<Box<Expression>>,
    #[serde(default)]
    pub never: Option<Box<Expression>>,
}

/// CharacterSetProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSetProperty {
    pub this: Box<Expression>,
    #[serde(default)]
    pub default: Option<Box<Expression>>,
}

/// ChecksumProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChecksumProperty {
    #[serde(default)]
    pub on: Option<Box<Expression>>,
    #[serde(default)]
    pub default: Option<Box<Expression>>,
}

/// CollateProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollateProperty {
    pub this: Box<Expression>,
    #[serde(default)]
    pub default: Option<Box<Expression>>,
}

/// DataBlocksizeProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataBlocksizeProperty {
    #[serde(default)]
    pub size: Option<i64>,
    #[serde(default)]
    pub units: Option<Box<Expression>>,
    #[serde(default)]
    pub minimum: Option<Box<Expression>>,
    #[serde(default)]
    pub maximum: Option<Box<Expression>>,
    #[serde(default)]
    pub default: Option<Box<Expression>>,
}

/// DataDeletionProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataDeletionProperty {
    /// Syntax marker for the ON/OFF keyword, not a transformable boolean expression.
    #[ast(skip)]
    pub on: Box<Expression>,
    #[serde(default)]
    pub filter_column: Option<Box<Expression>>,
    #[serde(default)]
    pub retention_period: Option<Box<Expression>>,
}

/// DefinerProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefinerProperty {
    pub this: Box<Expression>,
}

/// DistKeyProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistKeyProperty {
    pub this: Box<Expression>,
}

/// DistributedByProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistributedByProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    pub kind: String,
    #[serde(default)]
    pub buckets: Option<Box<Expression>>,
    #[serde(default)]
    pub order: Option<Box<Expression>>,
}

/// DistStyleProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistStyleProperty {
    pub this: Box<Expression>,
}

/// DuplicateKeyProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicateKeyProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// EngineProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineProperty {
    pub this: Box<Expression>,
}

/// ToTableProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToTableProperty {
    pub this: Box<Expression>,
}

/// ExecuteAsProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteAsProperty {
    pub this: Box<Expression>,
}

/// ExternalProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// FallbackProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FallbackProperty {
    #[serde(default)]
    pub no: Option<Box<Expression>>,
    #[serde(default)]
    pub protection: Option<Box<Expression>>,
}

/// FileFormatProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileFormatProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub hive_format: Option<Box<Expression>>,
}

/// CredentialsProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialsProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// FreespaceProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreespaceProperty {
    pub this: Box<Expression>,
    #[serde(default)]
    pub percent: Option<Box<Expression>>,
}

/// InheritsProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InheritsProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// InputModelProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputModelProperty {
    pub this: Box<Expression>,
}

/// OutputModelProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputModelProperty {
    pub this: Box<Expression>,
}

/// IsolatedLoadingProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IsolatedLoadingProperty {
    #[serde(default)]
    pub no: Option<Box<Expression>>,
    #[serde(default)]
    pub concurrent: Option<Box<Expression>>,
    #[serde(default)]
    pub target: Option<Box<Expression>>,
}

/// JournalProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalProperty {
    #[serde(default)]
    pub no: Option<Box<Expression>>,
    #[serde(default)]
    pub dual: Option<Box<Expression>>,
    #[serde(default)]
    pub before: Option<Box<Expression>>,
    #[serde(default)]
    pub local: Option<Box<Expression>>,
    #[serde(default)]
    pub after: Option<Box<Expression>>,
}

/// LanguageProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageProperty {
    pub this: Box<Expression>,
}

/// EnviromentProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnviromentProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// ClusteredByProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusteredByProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub sorted_by: Option<Box<Expression>>,
    #[serde(default)]
    pub buckets: Option<Box<Expression>>,
}

/// DictProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictProperty {
    pub this: Box<Expression>,
    pub kind: String,
    #[serde(default)]
    pub settings: Option<Box<Expression>>,
}

/// DictRange
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictRange {
    pub this: Box<Expression>,
    #[serde(default)]
    pub min: Option<Box<Expression>>,
    #[serde(default)]
    pub max: Option<Box<Expression>>,
}

/// OnCluster
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnCluster {
    pub this: Box<Expression>,
}

/// LikeProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LikeProperty {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// LocationProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationProperty {
    pub this: Box<Expression>,
}

/// LockProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockProperty {
    pub this: Box<Expression>,
}

/// LockingProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockingProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    pub kind: String,
    #[serde(default)]
    pub for_or_in: Option<Box<Expression>>,
    #[serde(default)]
    pub lock_type: Option<Box<Expression>>,
    #[serde(default)]
    pub override_: Option<Box<Expression>>,
}

/// LogProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogProperty {
    #[serde(default)]
    pub no: Option<Box<Expression>>,
}

/// MaterializedProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterializedProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// MergeBlockRatioProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeBlockRatioProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub no: Option<Box<Expression>>,
    #[serde(default)]
    pub default: Option<Box<Expression>>,
    #[serde(default)]
    pub percent: Option<Box<Expression>>,
}

/// OnProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnProperty {
    pub this: Box<Expression>,
}

/// OnCommitProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnCommitProperty {
    #[serde(default)]
    pub delete: Option<Box<Expression>>,
}

/// PartitionedByProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionedByProperty {
    pub this: Box<Expression>,
}

/// BigQuery PARTITION BY property in CREATE TABLE statements.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionByProperty {
    /// Structured partition scheme when the dialect supports methods and ranges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specification: Option<PartitionSpec>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// PartitionedByBucket
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionedByBucket {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// BigQuery CLUSTER BY property in CREATE TABLE statements.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterByColumnsProperty {
    #[serde(default)]
    pub columns: Vec<Identifier>,
}

/// PartitionByTruncate
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionByTruncate {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// PartitionByRangeProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionByRangeProperty {
    #[serde(default)]
    pub partition_expressions: Option<Box<Expression>>,
    #[serde(default)]
    pub create_expressions: Option<Box<Expression>>,
}

/// PartitionByRangePropertyDynamic
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionByRangePropertyDynamic {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub start: Option<Box<Expression>>,
    /// Use START/END/EVERY keywords (StarRocks) instead of FROM/TO/INTERVAL (Doris)
    #[serde(default)]
    pub use_start_end: bool,
    #[serde(default)]
    pub end: Option<Box<Expression>>,
    #[serde(default)]
    pub every: Option<Box<Expression>>,
}

/// PartitionByListProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionByListProperty {
    #[serde(default)]
    pub partition_expressions: Option<Box<Expression>>,
    #[serde(default)]
    pub create_expressions: Option<Box<Expression>>,
}

/// PartitionList
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionList {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Partition - represents PARTITION/SUBPARTITION clause
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Partition {
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub subpartition: bool,
}

/// Doris materialized-view REFRESH clause, such as REFRESH AUTO ON SCHEDULE EVERY 5 MINUTE.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefreshTriggerProperty {
    /// Method: COMPLETE or AUTO
    pub method: String,
    /// Trigger kind: MANUAL, COMMIT, or SCHEDULE
    #[serde(default)]
    pub kind: Option<String>,
    /// For SCHEDULE: EVERY n (the number)
    #[serde(default)]
    pub every: Option<Box<Expression>>,
    /// For SCHEDULE: the time unit (MINUTE, HOUR, DAY, etc.)
    #[serde(default)]
    pub unit: Option<String>,
    /// For SCHEDULE: STARTS 'datetime'
    #[serde(default)]
    pub starts: Option<Box<Expression>>,
}

/// UniqueKeyProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniqueKeyProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// RollupProperty - StarRocks ROLLUP (index_name(col1, col2), ...)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollupProperty {
    pub expressions: Vec<RollupIndex>,
}

/// RollupIndex - A single rollup index: name(col1, col2)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollupIndex {
    pub name: Identifier,
    pub expressions: Vec<Identifier>,
}

/// PartitionBoundSpec
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionBoundSpec {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub from_expressions: Option<Box<Expression>>,
    #[serde(default)]
    pub to_expressions: Option<Box<Expression>>,
}

/// PartitionedOfProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionedOfProperty {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// RemoteWithConnectionModelProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteWithConnectionModelProperty {
    pub this: Box<Expression>,
}

/// ReturnsProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReturnsProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub is_table: Option<Box<Expression>>,
    #[serde(default)]
    pub table: Option<Box<Expression>>,
    #[serde(default)]
    pub null: Option<Box<Expression>>,
}

/// RowFormatProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowFormatProperty {
    pub this: Box<Expression>,
}

/// RowFormatDelimitedProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowFormatDelimitedProperty {
    #[serde(default)]
    pub fields: Option<Box<Expression>>,
    #[serde(default)]
    pub escaped: Option<Box<Expression>>,
    #[serde(default)]
    pub collection_items: Option<Box<Expression>>,
    #[serde(default)]
    pub map_keys: Option<Box<Expression>>,
    #[serde(default)]
    pub lines: Option<Box<Expression>>,
    #[serde(default)]
    pub null: Option<Box<Expression>>,
    #[serde(default)]
    pub serde: Option<Box<Expression>>,
}

/// RowFormatSerdeProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowFormatSerdeProperty {
    pub this: Box<Expression>,
    #[serde(default)]
    pub serde_properties: Option<Box<Expression>>,
}

/// QueryTransform
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryTransform {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub command_script: Option<Box<Expression>>,
    #[serde(default)]
    pub schema: Option<Box<Expression>>,
    #[serde(default)]
    pub row_format_before: Option<Box<Expression>>,
    #[serde(default)]
    pub record_writer: Option<Box<Expression>>,
    #[serde(default)]
    pub row_format_after: Option<Box<Expression>>,
    #[serde(default)]
    pub record_reader: Option<Box<Expression>>,
}

/// SampleProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SampleProperty {
    pub this: Box<Expression>,
}

/// SecurityProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityProperty {
    pub this: Box<Expression>,
}

/// SchemaCommentProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaCommentProperty {
    pub this: Box<Expression>,
}

/// SemanticView
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticView {
    pub this: Box<Expression>,
    #[serde(default)]
    pub metrics: Option<Box<Expression>>,
    #[serde(default)]
    pub dimensions: Option<Box<Expression>>,
    #[serde(default)]
    pub facts: Option<Box<Expression>>,
    #[serde(default)]
    pub where_: Option<Box<Expression>>,
}

/// SerdeProperties
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SerdeProperties {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub with_: Option<Box<Expression>>,
}

/// SetProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetProperty {
    #[serde(default)]
    pub multi: Option<Box<Expression>>,
}

/// SharingProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharingProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// SetConfigProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetConfigProperty {
    pub this: Box<Expression>,
}

/// SettingsProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// SortKeyProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortKeyProperty {
    pub this: Box<Expression>,
    #[serde(default)]
    pub compound: Option<Box<Expression>>,
}

/// SqlReadWriteProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlReadWriteProperty {
    pub this: Box<Expression>,
}

/// SqlSecurityProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlSecurityProperty {
    pub this: Box<Expression>,
}

/// StabilityProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StabilityProperty {
    pub this: Box<Expression>,
}

/// StorageHandlerProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageHandlerProperty {
    pub this: Box<Expression>,
}

/// TemporaryProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporaryProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// Tags
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tags {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// TransformModelProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformModelProperty {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// TransientProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransientProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// UsingTemplateProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsingTemplateProperty {
    pub this: Box<Expression>,
}

/// ViewAttributeProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewAttributeProperty {
    pub this: Box<Expression>,
}

/// VolatileProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolatileProperty {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// WithDataProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithDataProperty {
    #[serde(default)]
    pub no: Option<Box<Expression>>,
    #[serde(default)]
    pub statistics: Option<Box<Expression>>,
}

/// WithJournalTableProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithJournalTableProperty {
    pub this: Box<Expression>,
}

/// WithSchemaBindingProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithSchemaBindingProperty {
    pub this: Box<Expression>,
}

/// WithSystemVersioningProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithSystemVersioningProperty {
    #[serde(default)]
    pub on: Option<Box<Expression>>,
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub data_consistency: Option<Box<Expression>>,
    #[serde(default)]
    pub retention_period: Option<Box<Expression>>,
    #[serde(default)]
    pub with_: Option<Box<Expression>>,
}

/// WithProcedureOptions
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithProcedureOptions {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// EncodeProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncodeProperty {
    pub this: Box<Expression>,
    #[serde(default)]
    pub properties: Vec<Expression>,
    #[serde(default)]
    pub key: Option<Box<Expression>>,
}

/// IncludeProperty
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncludeProperty {
    pub this: Box<Expression>,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub column_def: Option<Box<Expression>>,
}

/// Properties
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Properties {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Key/value pair in a BigQuery OPTIONS (...) clause.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionEntry {
    pub key: Identifier,
    pub value: Expression,
}

/// Typed BigQuery OPTIONS (...) property for CREATE TABLE and related DDL.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionsProperty {
    #[serde(default)]
    pub entries: Vec<OptionEntry>,
}

/// InputOutputFormat
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputOutputFormat {
    #[serde(default)]
    pub input_format: Option<Box<Expression>>,
    #[serde(default)]
    pub output_format: Option<Box<Expression>>,
}

/// Reference
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub options: Vec<Expression>,
}

/// QueryOption
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryOption {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// WithTableHint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithTableHint {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// IndexTableHint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexTableHint {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub target: Option<Box<Expression>>,
}
