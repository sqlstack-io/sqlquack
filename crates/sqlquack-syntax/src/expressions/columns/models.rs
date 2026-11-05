//! Data models of expressions columns.

use serde::Deserialize;
use serde::Serialize;

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::tables::models::ConstraintType;
use crate::expressions::tables::models::IndexKeyPart;
use crate::expressions::tables::models::OnCluster;
use crate::expressions::tables::models::SortOrder;
use crate::expressions::tables::models::Tags;
use crate::expressions::tables::models::TiDBAutoRandom;
use crate::expressions::tables::models::TiDBTableOption;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;

/// Column definition in CREATE TABLE
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: Identifier,
    pub data_type: DataType,
    pub nullable: Option<bool>,
    pub default: Option<Expression>,
    pub primary_key: bool,
    /// Sort order for PRIMARY KEY (ASC/DESC)
    #[serde(default)]
    pub primary_key_order: Option<SortOrder>,
    pub unique: bool,
    /// PostgreSQL 15+: UNIQUE NULLS NOT DISTINCT
    #[serde(default)]
    pub unique_nulls_not_distinct: bool,
    pub auto_increment: bool,
    /// TiDB distributed primary-key allocation attribute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_random: Option<TiDBAutoRandom>,
    pub comment: Option<String>,
    pub constraints: Vec<ColumnConstraint>,
    /// Track original order of constraints for accurate regeneration
    #[serde(default)]
    pub constraint_order: Vec<ConstraintType>,
    /// Teradata: FORMAT 'pattern'
    #[serde(default)]
    pub format: Option<String>,
    /// Teradata: TITLE 'title'
    #[serde(default)]
    pub title: Option<String>,
    /// Teradata: INLINE LENGTH n
    #[serde(default)]
    pub inline_length: Option<u64>,
    /// Teradata: COMPRESS or COMPRESS (values) or COMPRESS 'value'
    #[serde(default)]
    pub compress: Option<Vec<Expression>>,
    /// Teradata: CHARACTER SET name
    #[serde(default)]
    pub character_set: Option<String>,
    /// Teradata: UPPERCASE
    #[serde(default)]
    pub uppercase: bool,
    /// Teradata: CASESPECIFIC / NOT CASESPECIFIC (None = not specified, Some(true) = CASESPECIFIC, Some(false) = NOT CASESPECIFIC)
    #[serde(default)]
    pub casespecific: Option<bool>,
    /// Snowflake: AUTOINCREMENT START value
    #[serde(default)]
    pub auto_increment_start: Option<Box<Expression>>,
    /// Snowflake: AUTOINCREMENT INCREMENT value
    #[serde(default)]
    pub auto_increment_increment: Option<Box<Expression>>,
    /// Snowflake: AUTOINCREMENT ORDER/NOORDER (true = ORDER, false = NOORDER, None = not specified)
    #[serde(default)]
    pub auto_increment_order: Option<bool>,
    /// MySQL: UNSIGNED modifier
    #[serde(default)]
    pub unsigned: bool,
    /// MySQL: ZEROFILL modifier
    #[serde(default)]
    pub zerofill: bool,
    /// MySQL: ON UPDATE expression (e.g., ON UPDATE CURRENT_TIMESTAMP)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_update: Option<Expression>,
    /// MySQL: column VISIBLE/INVISIBLE modifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    /// Named constraint for UNIQUE (e.g., CONSTRAINT must_be_different UNIQUE)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_constraint_name: Option<String>,
    /// Named constraint for NOT NULL (e.g., CONSTRAINT present NOT NULL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_null_constraint_name: Option<String>,
    /// Named constraint for PRIMARY KEY (e.g., CONSTRAINT pk_name PRIMARY KEY)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_key_constraint_name: Option<String>,
    /// Named constraint for CHECK (e.g., CONSTRAINT chk_name CHECK(...))
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_constraint_name: Option<String>,
    /// BigQuery: OPTIONS (key=value, ...) on column
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Expression>,
    /// SQLite: Column definition without explicit type
    #[serde(default)]
    pub no_type: bool,
    /// Redshift: ENCODE encoding_type (e.g., ZSTD, DELTA, LZO, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    /// ClickHouse: CODEC(LZ4HC(9), ZSTD, DELTA)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
    /// ClickHouse: EPHEMERAL [expr] modifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<Option<Box<Expression>>>,
    /// ClickHouse: MATERIALIZED expr modifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materialized_expr: Option<Box<Expression>>,
    /// ClickHouse: ALIAS expr modifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias_expr: Option<Box<Expression>>,
    /// ClickHouse: TTL expr modifier on columns
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_expr: Option<Box<Expression>>,
    /// TSQL: NOT FOR REPLICATION
    #[serde(default)]
    pub not_for_replication: bool,
}

impl ColumnDef {
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: Identifier::new(name),
            data_type,
            nullable: None,
            default: None,
            primary_key: false,
            primary_key_order: None,
            unique: false,
            unique_nulls_not_distinct: false,
            auto_increment: false,
            auto_random: None,
            comment: None,
            constraints: Vec::new(),
            constraint_order: Vec::new(),
            format: None,
            title: None,
            inline_length: None,
            compress: None,
            character_set: None,
            uppercase: false,
            casespecific: None,
            auto_increment_start: None,
            auto_increment_increment: None,
            auto_increment_order: None,
            unsigned: false,
            zerofill: false,
            on_update: None,
            visible: None,
            unique_constraint_name: None,
            not_null_constraint_name: None,
            primary_key_constraint_name: None,
            check_constraint_name: None,
            options: Vec::new(),
            no_type: false,
            encoding: None,
            codec: None,
            ephemeral: None,
            materialized_expr: None,
            alias_expr: None,
            ttl_expr: None,
            not_for_replication: false,
        }
    }
}

/// Column-level constraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnConstraint {
    NotNull,
    Null,
    Unique,
    PrimaryKey,
    Default(Expression),
    Check(Expression),
    References(Box<ForeignKeyRef>),
    GeneratedAsIdentity(GeneratedAsIdentity),
    Collate(Identifier),
    Comment(String),
    /// Snowflake: TAG (key='value', ...)
    Tags(Tags),
    /// Generated column: GENERATED ALWAYS AS (expr) STORED|VIRTUAL, or TSQL AS (expr) PERSISTED.
    ComputedColumn(ComputedColumn),
    /// TSQL temporal: GENERATED ALWAYS AS ROW START|END [HIDDEN]
    GeneratedAsRow(GeneratedAsRow),
    /// PATH constraint for XMLTABLE/JSON_TABLE columns: PATH 'xpath'
    Path(Expression),
}

/// Computed/generated column constraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputedColumn {
    /// The expression that computes the column value
    pub expression: Box<Expression>,
    /// PERSISTED (TSQL) or STORED (MySQL/PostgreSQL) = true; VIRTUAL = false; None = not specified
    #[serde(default)]
    pub persisted: bool,
    /// NOT NULL (TSQL computed columns)
    #[serde(default)]
    pub not_null: bool,
    /// Persistence keyword (STORED, VIRTUAL or PERSISTED); `None` uses the dialect's default.
    #[serde(default)]
    pub persistence_kind: Option<String>,
    /// Optional data type for SingleStore: AS (expr) PERSISTED TYPE NOT NULL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_type: Option<DataType>,
}

/// TSQL temporal column constraint: GENERATED ALWAYS AS ROW START|END [HIDDEN]
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedAsRow {
    /// true = ROW START, false = ROW END
    pub start: bool,
    /// HIDDEN modifier
    #[serde(default)]
    pub hidden: bool,
}

/// Generated identity column constraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedAsIdentity {
    /// True for ALWAYS, False for BY DEFAULT
    pub always: bool,
    /// ON NULL (only valid with BY DEFAULT)
    pub on_null: bool,
    /// START WITH value
    pub start: Option<Box<Expression>>,
    /// INCREMENT BY value
    pub increment: Option<Box<Expression>>,
    /// MINVALUE
    pub minvalue: Option<Box<Expression>>,
    /// MAXVALUE
    pub maxvalue: Option<Box<Expression>>,
    /// CYCLE option - Some(true) = CYCLE, Some(false) = NO CYCLE, None = not specified
    pub cycle: Option<bool>,
}

/// Constraint modifiers (shared between table-level constraints)
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Default, PartialEq, Serialize, Deserialize,
)]
pub struct ConstraintModifiers {
    /// ENFORCED / NOT ENFORCED
    pub enforced: Option<bool>,
    /// DEFERRABLE / NOT DEFERRABLE
    pub deferrable: Option<bool>,
    /// INITIALLY DEFERRED / INITIALLY IMMEDIATE
    pub initially_deferred: Option<bool>,
    /// NORELY (Oracle)
    pub norely: bool,
    /// RELY (Oracle)
    pub rely: bool,
    /// USING index type (MySQL): BTREE or HASH
    #[serde(default)]
    pub using: Option<String>,
    /// True if USING appeared before columns (MySQL: INDEX USING BTREE (col) vs INDEX (col) USING BTREE)
    #[serde(default)]
    pub using_before_columns: bool,
    /// MySQL index COMMENT 'text'
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// MySQL index VISIBLE/INVISIBLE
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    /// MySQL ENGINE_ATTRIBUTE = 'value'
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_attribute: Option<String>,
    /// MySQL WITH PARSER name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_parser: Option<String>,
    /// PostgreSQL NOT VALID (constraint is not validated against existing data)
    #[serde(default)]
    pub not_valid: bool,
    /// TSQL CLUSTERED/NONCLUSTERED modifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clustered: Option<String>,
    /// SQLite ON CONFLICT clause: ROLLBACK, ABORT, FAIL, IGNORE, or REPLACE
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_conflict: Option<String>,
    /// TSQL WITH options (e.g., PAD_INDEX=ON, STATISTICS_NORECOMPUTE=OFF)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with_options: Vec<(String, String)>,
    /// TSQL ON filegroup (e.g., ON [INDEX], ON [PRIMARY])
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_filegroup: Option<Identifier>,
}

/// Table-level constraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableConstraint {
    PrimaryKey {
        name: Option<Identifier>,
        columns: Vec<Identifier>,
        /// Structured quoted, prefixed or ordered keys; authoritative over `columns` when set.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        key_parts: Vec<IndexKeyPart>,
        /// Databricks primary-key columns marked with the TIMESERIES attribute.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        timeseries_columns: Vec<Identifier>,
        /// INCLUDE (columns) - non-key columns included in the index (PostgreSQL)
        #[serde(default)]
        include_columns: Vec<Identifier>,
        #[serde(default)]
        modifiers: ConstraintModifiers,
        /// Whether the CONSTRAINT keyword was used (vs MySQL's `PRIMARY KEY name (cols)` syntax)
        #[serde(default)]
        has_constraint_keyword: bool,
    },
    Unique {
        name: Option<Identifier>,
        columns: Vec<Identifier>,
        /// Structured quoted, prefixed or ordered keys, as for `PrimaryKey`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        key_parts: Vec<IndexKeyPart>,
        /// Whether columns are parenthesized (false for UNIQUE idx_name without parens)
        #[serde(default)]
        columns_parenthesized: bool,
        #[serde(default)]
        modifiers: ConstraintModifiers,
        /// Whether the CONSTRAINT keyword was used (vs MySQL's `UNIQUE name (cols)` syntax)
        #[serde(default)]
        has_constraint_keyword: bool,
        /// PostgreSQL 15+: NULLS NOT DISTINCT
        #[serde(default)]
        nulls_not_distinct: bool,
    },
    ForeignKey {
        name: Option<Identifier>,
        columns: Vec<Identifier>,
        #[serde(default)]
        references: Option<Box<ForeignKeyRef>>,
        /// ON DELETE action when REFERENCES is absent
        #[serde(default)]
        on_delete: Option<ReferentialAction>,
        /// ON UPDATE action when REFERENCES is absent
        #[serde(default)]
        on_update: Option<ReferentialAction>,
        #[serde(default)]
        modifiers: ConstraintModifiers,
    },
    Check {
        name: Option<Identifier>,
        expression: Expression,
        #[serde(default)]
        modifiers: ConstraintModifiers,
    },
    /// ClickHouse ASSUME constraint (query optimization assumption)
    Assume {
        name: Option<Identifier>,
        expression: Expression,
    },
    /// TSQL named DEFAULT constraint: CONSTRAINT name DEFAULT value FOR column
    Default {
        name: Option<Identifier>,
        expression: Expression,
        column: Identifier,
    },
    /// INDEX / KEY constraint (MySQL)
    Index {
        name: Option<Identifier>,
        columns: Vec<Identifier>,
        /// Expression key parts, for functional index keys `columns` cannot hold.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        key_parts: Vec<IndexKeyPart>,
        /// Index kind: UNIQUE, FULLTEXT, SPATIAL, etc.
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        modifiers: ConstraintModifiers,
        /// True if KEY keyword was used instead of INDEX
        #[serde(default)]
        use_key_keyword: bool,
        /// ClickHouse: indexed expression (instead of columns)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expression: Option<Box<Expression>>,
        /// ClickHouse: TYPE type_func(args)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index_type: Option<Box<Expression>>,
        /// ClickHouse: GRANULARITY n
        #[serde(default, skip_serializing_if = "Option::is_none")]
        granularity: Option<Box<Expression>>,
    },
    /// ClickHouse PROJECTION definition
    Projection {
        name: Identifier,
        expression: Expression,
    },
    /// PostgreSQL LIKE clause: LIKE source_table [INCLUDING|EXCLUDING options]
    Like {
        source: TableRef,
        /// Options as (INCLUDING|EXCLUDING, property) pairs
        options: Vec<(LikeOptionAction, String)>,
    },
    /// TSQL PERIOD FOR SYSTEM_TIME (start_col, end_col)
    PeriodForSystemTime {
        start_col: Identifier,
        end_col: Identifier,
    },
    /// PostgreSQL EXCLUDE [USING method] (element WITH operator, ...) constraint.
    Exclude {
        name: Option<Identifier>,
        /// Index access method (gist, btree, etc.)
        #[serde(default)]
        using: Option<String>,
        /// Elements: (expression, operator) pairs
        elements: Vec<ExcludeElement>,
        /// INCLUDE columns
        #[serde(default)]
        include_columns: Vec<Identifier>,
        /// WHERE predicate
        #[serde(default)]
        where_clause: Option<Box<Expression>>,
        /// WITH (storage_parameters)
        #[serde(default)]
        with_params: Vec<(String, String)>,
        /// USING INDEX TABLESPACE tablespace_name
        #[serde(default)]
        using_index_tablespace: Option<String>,
        #[serde(default)]
        modifiers: ConstraintModifiers,
    },
    /// Snowflake TAG clause: TAG (key='value', key2='value2')
    Tags(Tags),
    /// PostgreSQL table-level INITIALLY DEFERRED/IMMEDIATE default for deferrable constraints.
    InitiallyDeferred {
        /// true = INITIALLY DEFERRED, false = INITIALLY IMMEDIATE
        deferred: bool,
    },
}

/// Element in an EXCLUDE constraint: expression WITH operator
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExcludeElement {
    /// The column expression (may include operator class, ordering, nulls)
    pub expression: String,
    /// The operator (e.g., &&, =)
    pub operator: String,
}

/// Action for LIKE clause options
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum LikeOptionAction {
    Including,
    Excluding,
}

/// MATCH type for foreign keys
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum MatchType {
    Full,
    Partial,
    Simple,
}

/// Foreign key reference
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKeyRef {
    pub table: TableRef,
    pub columns: Vec<Identifier>,
    pub on_delete: Option<ReferentialAction>,
    pub on_update: Option<ReferentialAction>,
    /// True if ON UPDATE appears before ON DELETE in the original SQL
    #[serde(default)]
    pub on_update_first: bool,
    /// MATCH clause (FULL, PARTIAL, SIMPLE)
    #[serde(default)]
    pub match_type: Option<MatchType>,
    /// True if MATCH appears after ON DELETE/ON UPDATE clauses
    #[serde(default)]
    pub match_after_actions: bool,
    /// CONSTRAINT name (e.g., CONSTRAINT fk_name REFERENCES ...)
    #[serde(default)]
    pub constraint_name: Option<String>,
    /// DEFERRABLE / NOT DEFERRABLE
    #[serde(default)]
    pub deferrable: Option<bool>,
    /// Snowflake: FOREIGN KEY REFERENCES (includes FOREIGN KEY keywords before REFERENCES)
    #[serde(default)]
    pub has_foreign_key_keywords: bool,
}

/// Referential action for foreign keys
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum ReferentialAction {
    Cascade,
    SetNull,
    SetDefault,
    Restrict,
    NoAction,
}

/// ALTER TABLE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterTable {
    pub name: TableRef,
    pub actions: Vec<AlterTableAction>,
    /// IF EXISTS clause
    #[serde(default)]
    pub if_exists: bool,
    /// MySQL: ALGORITHM=INPLACE|COPY|DEFAULT|INSTANT
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    /// MySQL: LOCK=NONE|SHARED|DEFAULT|EXCLUSIVE
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock: Option<String>,
    /// TSQL: WITH CHECK / WITH NOCHECK modifier before ADD CONSTRAINT
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_check: Option<String>,
    /// Hive: PARTITION clause before actions (e.g., ALTER TABLE x PARTITION(y=z) ADD COLUMN ...)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<Vec<(Identifier, Expression)>>,
    /// ClickHouse: ON CLUSTER clause for distributed DDL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_cluster: Option<OnCluster>,
    /// Snowflake: ALTER ICEBERG TABLE
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table_modifier: Option<String>,
}

impl AlterTable {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            actions: Vec::new(),
            if_exists: false,
            algorithm: None,
            lock: None,
            with_check: None,
            partition: None,
            on_cluster: None,
            table_modifier: None,
        }
    }
}

/// Column position for ADD COLUMN (MySQL/MariaDB)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnPosition {
    First,
    After(Identifier),
}

/// Snowflake `{ ADD | DROP | MODIFY } DATA METRIC FUNCTION <name> ON ( ... ) [ <clause> ], ...`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataMetricFunctionAction {
    /// `ADD`, `DROP` or `MODIFY`.
    pub kind: String,
    pub items: Vec<DataMetricFunctionItem>,
}

/// One `<name> ON ( <argument>, ... )` of a data metric function action, with its clauses.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataMetricFunctionItem {
    pub name: TableRef,
    pub arguments: Vec<Expression>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clauses: Vec<DataMetricFunctionClause>,
}

/// A clause after a data metric function's arguments.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataMetricFunctionClause {
    /// `SENSITIVITY = <value>` or `ANOMALY_DETECTION = <value>`.
    Property {
        name: String,
        value: Expression,
    },
    /// `[ ADD | DROP | MODIFY ] EXPECTATION <expectation>, ...`.
    Expectations {
        action: Option<String>,
        expectations: Vec<DataMetricExpectation>,
    },
    /// `WITHIN GROUP ( <column>, ... ) [ GROUP LIMIT <n> ]`.
    WithinGroup {
        columns: Vec<Expression>,
        limit: Option<Expression>,
    },
    ExecuteAsRole(Identifier),
    /// `FILTER ( [ <condition> ] )`.
    Filter(Option<Expression>),
    /// `SUSPEND` or `RESUME`.
    State(String),
}

/// A data metric function expectation: its name and, except under DROP, its condition.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataMetricExpectation {
    pub name: Identifier,
    pub condition: Option<Expression>,
}

/// Actions for ALTER TABLE
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterTableAction {
    DataMetricFunction(DataMetricFunctionAction),
    AddColumn {
        column: ColumnDef,
        if_not_exists: bool,
        position: Option<ColumnPosition>,
    },
    DropColumn {
        name: Identifier,
        if_exists: bool,
        cascade: bool,
    },
    RenameColumn {
        old_name: Identifier,
        new_name: Identifier,
        if_exists: bool,
    },
    AlterColumn {
        name: Identifier,
        action: AlterColumnAction,
        /// Whether this was parsed from MODIFY COLUMN syntax (MySQL)
        #[serde(default)]
        use_modify_keyword: bool,
    },
    /// MySQL/TiDB `MODIFY [COLUMN]` with a complete replacement column definition.
    ModifyColumn {
        column: ColumnDef,
        if_exists: bool,
        position: Option<ColumnPosition>,
    },
    RenameTable(TableRef),
    AddConstraint(TableConstraint),
    DropConstraint {
        name: Identifier,
        if_exists: bool,
    },
    /// DROP FOREIGN KEY action (Oracle/MySQL): ALTER TABLE t DROP FOREIGN KEY fk_name
    DropForeignKey {
        name: Identifier,
    },
    /// DROP PARTITION action (Hive/BigQuery)
    DropPartition {
        /// List of partitions to drop (each partition is a list of key=value pairs)
        partitions: Vec<Vec<(Identifier, Expression)>>,
        if_exists: bool,
    },
    /// ADD PARTITION action (Hive/Spark)
    AddPartition {
        /// The partition expression
        partition: Expression,
        if_not_exists: bool,
        location: Option<Expression>,
    },
    /// DELETE action (BigQuery): ALTER TABLE t DELETE WHERE condition
    Delete {
        where_clause: Expression,
    },
    /// SWAP WITH action (Snowflake): ALTER TABLE a SWAP WITH b
    SwapWith(TableRef),
    /// SET property action (Snowflake): ALTER TABLE t SET property=value
    SetProperty {
        properties: Vec<(String, Expression)>,
    },
    /// UNSET property action (Snowflake): ALTER TABLE t UNSET property
    UnsetProperty {
        properties: Vec<String>,
    },
    /// CLUSTER BY action (Snowflake): ALTER TABLE t CLUSTER BY (col1, col2)
    ClusterBy {
        expressions: Vec<Expression>,
    },
    /// SET TAG action (Snowflake): ALTER TABLE t SET TAG key='value'
    SetTag {
        expressions: Vec<(String, Expression)>,
    },
    /// UNSET TAG action (Snowflake): ALTER TABLE t UNSET TAG key1, key2
    UnsetTag {
        names: Vec<String>,
    },
    /// SET with parenthesized options (TSQL): ALTER TABLE t SET (SYSTEM_VERSIONING=ON, ...)
    SetOptions {
        expressions: Vec<Expression>,
    },
    /// ALTER INDEX action (MySQL): ALTER TABLE t ALTER INDEX i VISIBLE/INVISIBLE
    AlterIndex {
        name: Identifier,
        visible: bool,
    },
    /// PostgreSQL: ALTER TABLE t SET LOGGED/UNLOGGED/WITHOUT CLUSTER/WITHOUT OIDS/ACCESS METHOD/TABLESPACE
    SetAttribute {
        attribute: String,
    },
    /// Snowflake: ALTER TABLE t SET STAGE_FILE_FORMAT = (options)
    SetStageFileFormat {
        options: Option<Expression>,
    },
    /// Snowflake: ALTER TABLE t SET STAGE_COPY_OPTIONS = (options)
    SetStageCopyOptions {
        options: Option<Expression>,
    },
    /// Hive/Spark: ADD COLUMNS (col1 TYPE, col2 TYPE) [CASCADE]
    AddColumns {
        columns: Vec<ColumnDef>,
        cascade: bool,
    },
    /// Spark/Databricks: DROP COLUMNS (col1, col2, ...)
    DropColumns {
        names: Vec<Identifier>,
    },
    /// CHANGE [COLUMN] old new [type] [COMMENT '...'] (Hive/MySQL/SingleStore).
    ChangeColumn {
        old_name: Identifier,
        new_name: Identifier,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data_type: Option<DataType>,
        comment: Option<String>,
        #[serde(default)]
        cascade: bool,
    },
    /// Redshift ALTER [COMPOUND] SORTKEY AUTO|NONE|(cols).
    AlterSortKey {
        /// AUTO or NONE keyword
        this: Option<String>,
        /// Column list for (col1, col2) syntax
        expressions: Vec<Expression>,
        /// Whether COMPOUND keyword was present
        compound: bool,
    },
    /// Redshift ALTER DISTSTYLE ALL|EVEN|AUTO|KEY [DISTKEY col], or ALTER DISTKEY col.
    AlterDistStyle {
        /// Distribution style: ALL, EVEN, AUTO, or KEY
        style: String,
        /// DISTKEY column (only when style is KEY)
        distkey: Option<Identifier>,
    },
    /// Redshift: ALTER TABLE t SET TABLE PROPERTIES ('a' = '5', 'b' = 'c')
    SetTableProperties {
        properties: Vec<(Expression, Expression)>,
    },
    /// Redshift: ALTER TABLE t SET LOCATION 's3://bucket/folder/'
    SetLocation {
        location: String,
    },
    /// Redshift: ALTER TABLE t SET FILE FORMAT AVRO
    SetFileFormat {
        format: String,
    },
    /// ClickHouse: ALTER TABLE t REPLACE PARTITION expr FROM source_table
    ReplacePartition {
        partition: Expression,
        source: Option<Box<Expression>>,
    },
    /// Set a TiDB-specific table option. `force` is valid for `AUTO_RANDOM_BASE`.
    SetTiDBTableOption {
        option: TiDBTableOption,
        #[serde(default)]
        force: bool,
    },
    /// TiDB `REMOVE TTL`.
    RemoveTiDBTtl {
        #[serde(default)]
        executable_comment: bool,
    },
    /// Raw SQL for dialect-specific ALTER TABLE actions (e.g., ClickHouse UPDATE/DELETE/DETACH/etc.)
    Raw {
        sql: String,
    },
}

/// Actions for ALTER COLUMN
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterColumnAction {
    SetDataType {
        data_type: DataType,
        /// USING expression for type conversion (PostgreSQL)
        using: Option<Expression>,
        /// COLLATE clause (TSQL: ALTER COLUMN col TYPE COLLATE collation_name)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        collate: Option<String>,
    },
    SetDefault(Expression),
    DropDefault,
    SetNotNull,
    DropNotNull,
    /// Set column comment
    Comment(String),
    /// MySQL: SET VISIBLE
    SetVisible,
    /// MySQL: SET INVISIBLE
    SetInvisible,
}

/// AlterColumn
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterColumn {
    pub this: Box<Expression>,
    #[serde(default)]
    pub dtype: Option<Box<Expression>>,
    #[serde(default)]
    pub collate: Option<Box<Expression>>,
    #[serde(default)]
    pub using: Option<Box<Expression>>,
    #[serde(default)]
    pub default: Option<Box<Expression>>,
    #[serde(default)]
    pub drop: Option<Box<Expression>>,
    #[serde(default)]
    pub comment: Option<Box<Expression>>,
    #[serde(default)]
    pub allow_null: Option<Box<Expression>>,
    #[serde(default)]
    pub visible: Option<Box<Expression>>,
    #[serde(default)]
    pub rename_to: Option<Box<Expression>>,
}

/// AlterSortKey
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterSortKey {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub compound: Option<Box<Expression>>,
}

/// AlterSet
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterSet {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub option: Option<Box<Expression>>,
    #[serde(default)]
    pub tablespace: Option<Box<Expression>>,
    #[serde(default)]
    pub access_method: Option<Box<Expression>>,
    #[serde(default)]
    pub file_format: Option<Box<Expression>>,
    #[serde(default)]
    pub copy_options: Option<Box<Expression>>,
    #[serde(default)]
    pub tag: Option<Box<Expression>>,
    #[serde(default)]
    pub location: Option<Box<Expression>>,
    #[serde(default)]
    pub serde: Option<Box<Expression>>,
}

/// RenameColumn
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameColumn {
    pub this: Box<Expression>,
    #[serde(default)]
    pub to: Option<Box<Expression>>,
    #[serde(default)]
    pub exists: bool,
}

/// Comprehension
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comprehension {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub position: Option<Box<Expression>>,
    #[serde(default)]
    pub iterator: Option<Box<Expression>>,
    #[serde(default)]
    pub condition: Option<Box<Expression>>,
}

/// MergeTreeTTLAction
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeTreeTTLAction {
    pub this: Box<Expression>,
    #[serde(default)]
    pub delete: Option<Box<Expression>>,
    #[serde(default)]
    pub recompress: Option<Box<Expression>>,
    #[serde(default)]
    pub to_disk: Option<Box<Expression>>,
    #[serde(default)]
    pub to_volume: Option<Box<Expression>>,
}

/// MergeTreeTTL
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeTreeTTL {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub where_: Option<Box<Expression>>,
    #[serde(default)]
    pub group: Option<Box<Expression>>,
    #[serde(default)]
    pub aggregates: Option<Box<Expression>>,
}

/// IndexConstraintOption
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexConstraintOption {
    #[serde(default)]
    pub key_block_size: Option<Box<Expression>>,
    #[serde(default)]
    pub using: Option<Box<Expression>>,
    #[serde(default)]
    pub parser: Option<Box<Expression>>,
    #[serde(default)]
    pub comment: Option<Box<Expression>>,
    #[serde(default)]
    pub visible: Option<Box<Expression>>,
    #[serde(default)]
    pub engine_attr: Option<Box<Expression>>,
    #[serde(default)]
    pub secondary_engine_attr: Option<Box<Expression>>,
}

/// PeriodForSystemTimeConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeriodForSystemTimeConstraint {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// CaseSpecificColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseSpecificColumnConstraint {
    #[serde(default)]
    pub not_: Option<Box<Expression>>,
}

/// CharacterSetColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSetColumnConstraint {
    pub this: Box<Expression>,
}

/// CheckColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckColumnConstraint {
    pub this: Box<Expression>,
    #[serde(default)]
    pub enforced: Option<Box<Expression>>,
}

/// AssumeColumnConstraint (ClickHouse ASSUME constraint)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssumeColumnConstraint {
    pub this: Box<Expression>,
}

/// CompressColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressColumnConstraint {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// DateFormatColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateFormatColumnConstraint {
    pub this: Box<Expression>,
}

/// EphemeralColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralColumnConstraint {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// WithOperator
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithOperator {
    pub this: Box<Expression>,
    pub op: String,
}

/// GeneratedAsIdentityColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedAsIdentityColumnConstraint {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub on_null: Option<Box<Expression>>,
    #[serde(default)]
    pub start: Option<Box<Expression>>,
    #[serde(default)]
    pub increment: Option<Box<Expression>>,
    #[serde(default)]
    pub minvalue: Option<Box<Expression>>,
    #[serde(default)]
    pub maxvalue: Option<Box<Expression>>,
    #[serde(default)]
    pub cycle: Option<Box<Expression>>,
    #[serde(default)]
    pub order: Option<Box<Expression>>,
}

/// MySQL/TSQL auto-increment column marker.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoIncrementColumnConstraint;

/// CommentColumnConstraint - Column comment marker
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentColumnConstraint;

/// GeneratedAsRowColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedAsRowColumnConstraint {
    #[serde(default)]
    pub start: Option<Box<Expression>>,
    #[serde(default)]
    pub hidden: Option<Box<Expression>>,
}

/// IndexColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexColumnConstraint {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub index_type: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub granularity: Option<Box<Expression>>,
}

/// MaskingPolicyColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaskingPolicyColumnConstraint {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// NotNullColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotNullColumnConstraint {
    #[serde(default)]
    pub allow_null: Option<Box<Expression>>,
}

/// DefaultColumnConstraint - DEFAULT value for a column
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultColumnConstraint {
    pub this: Box<Expression>,
    /// TSQL: DEFAULT value FOR column (table-level default constraint)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub for_column: Option<Identifier>,
}

/// PrimaryKeyColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrimaryKeyColumnConstraint {
    #[serde(default)]
    pub desc: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
}

/// UniqueColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UniqueColumnConstraint {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub index_type: Option<Box<Expression>>,
    #[serde(default)]
    pub on_conflict: Option<Box<Expression>>,
    #[serde(default)]
    pub nulls: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
}

/// WatermarkColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatermarkColumnConstraint {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// ComputedColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputedColumnConstraint {
    pub this: Box<Expression>,
    #[serde(default)]
    pub persisted: Option<Box<Expression>>,
    #[serde(default)]
    pub not_null: Option<Box<Expression>>,
    #[serde(default)]
    pub data_type: Option<Box<Expression>>,
}

/// InOutColumnConstraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InOutColumnConstraint {
    #[serde(default)]
    pub input_: Option<Box<Expression>>,
    #[serde(default)]
    pub output: Option<Box<Expression>>,
}

/// PathColumnConstraint - PATH 'xpath' for XMLTABLE/JSON_TABLE columns
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathColumnConstraint {
    pub this: Box<Expression>,
}

/// Constraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}
