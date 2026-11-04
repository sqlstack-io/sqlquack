//! Data models of expressions schema objects.

use serde::Deserialize;
use serde::Serialize;

use crate::expressions::columns::models::AlterColumnAction;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::schema_objects::_helpers::serde_defaults::default_true;
use crate::expressions::schema_objects::_helpers::serde_defaults::is_true;
use crate::expressions::statements::models::Schema;
use crate::expressions::tables::models::OnCluster;
use crate::expressions::tables::models::RefreshTriggerProperty;
use crate::expressions::tables::models::UniqueKeyProperty;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::StructField;

/// View column definition with optional COMMENT and OPTIONS (BigQuery)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewColumn {
    pub name: Identifier,
    pub comment: Option<String>,
    /// BigQuery: OPTIONS (key=value, ...) on column
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Expression>,
}

impl ViewColumn {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            comment: None,
            options: Vec::new(),
        }
    }

    pub fn with_comment(name: impl Into<String>, comment: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            comment: Some(comment.into()),
            options: Vec::new(),
        }
    }
}

/// CREATE VIEW statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateView {
    pub name: TableRef,
    pub columns: Vec<ViewColumn>,
    pub query: Expression,
    pub or_replace: bool,
    /// TSQL: CREATE OR ALTER
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub or_alter: bool,
    pub if_not_exists: bool,
    pub materialized: bool,
    pub temporary: bool,
    /// Snowflake: SECURE VIEW
    #[serde(default)]
    pub secure: bool,
    /// MySQL: ALGORITHM=UNDEFINED/MERGE/TEMPTABLE
    #[serde(skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    /// MySQL: DEFINER=user@host
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definer: Option<String>,
    /// MySQL: SQL SECURITY DEFINER/INVOKER; Presto: SECURITY DEFINER/INVOKER
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<FunctionSecurity>,
    /// True for MySQL-style "SQL SECURITY", false for Presto-style "SECURITY"
    #[serde(default = "default_true")]
    pub security_sql_style: bool,
    /// True when SQL SECURITY appears after the view name (not before VIEW keyword)
    #[serde(default)]
    pub security_after_name: bool,
    /// Whether the query was parenthesized: AS (SELECT ...)
    #[serde(default)]
    pub query_parenthesized: bool,
    /// Teradata: LOCKING mode (ROW, TABLE, DATABASE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locking_mode: Option<String>,
    /// Teradata: LOCKING access type (ACCESS, READ, WRITE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locking_access: Option<String>,
    /// Snowflake: COPY GRANTS
    #[serde(default)]
    pub copy_grants: bool,
    /// Snowflake: COMMENT = 'text'
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub comment: Option<String>,
    /// Snowflake: WITH ROW ACCESS POLICY ... clause
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub row_access_policy: Option<String>,
    /// Snowflake: TAG (name='value', ...)
    #[serde(default)]
    pub tags: Vec<(String, String)>,
    /// BigQuery: OPTIONS (key=value, ...)
    #[serde(default)]
    pub options: Vec<Expression>,
    /// Doris: BUILD IMMEDIATE/DEFERRED for materialized views
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub build: Option<String>,
    /// Doris: REFRESH property for materialized views
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub refresh: Option<Box<RefreshTriggerProperty>>,
    /// Doris typed column definitions of a materialized view, used instead of `columns`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub schema: Option<Box<Schema>>,
    /// Doris: KEY (columns) for materialized views
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unique_key: Option<Box<UniqueKeyProperty>>,
    /// Redshift: WITH NO SCHEMA BINDING
    #[serde(default)]
    pub no_schema_binding: bool,
    /// Redshift: AUTO REFRESH YES|NO for materialized views
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub auto_refresh: Option<bool>,
    /// ClickHouse: POPULATE / EMPTY before AS in materialized views
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub clickhouse_population: Option<String>,
    /// ClickHouse: ON CLUSTER clause
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_cluster: Option<OnCluster>,
    /// ClickHouse: TO destination_table
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_table: Option<TableRef>,
    /// ClickHouse: Table properties (ENGINE, ORDER BY, SAMPLE, SETTINGS, TTL, etc.) for materialized views
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table_properties: Vec<Expression>,
}

impl CreateView {
    pub fn new(name: impl Into<String>, query: Expression) -> Self {
        Self {
            name: TableRef::new(name),
            columns: Vec::new(),
            query,
            or_replace: false,
            or_alter: false,
            if_not_exists: false,
            materialized: false,
            temporary: false,
            secure: false,
            algorithm: None,
            definer: None,
            security: None,
            security_sql_style: true,
            security_after_name: false,
            query_parenthesized: false,
            locking_mode: None,
            locking_access: None,
            copy_grants: false,
            comment: None,
            row_access_policy: None,
            tags: Vec::new(),
            options: Vec::new(),
            build: None,
            refresh: None,
            schema: None,
            unique_key: None,
            no_schema_binding: false,
            auto_refresh: None,
            clickhouse_population: None,
            on_cluster: None,
            to_table: None,
            table_properties: Vec::new(),
        }
    }
}

/// DROP VIEW statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropView {
    pub name: TableRef,
    pub if_exists: bool,
    pub materialized: bool,
}

impl DropView {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            if_exists: false,
            materialized: false,
        }
    }
}

/// ALTER VIEW statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterView {
    pub name: TableRef,
    pub actions: Vec<AlterViewAction>,
    /// MySQL: ALGORITHM = MERGE|TEMPTABLE|UNDEFINED
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    /// MySQL: DEFINER = 'user'@'host'
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definer: Option<String>,
    /// MySQL: SQL SECURITY = DEFINER|INVOKER
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sql_security: Option<String>,
    /// TSQL: WITH option (SCHEMABINDING, ENCRYPTION, VIEW_METADATA)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_option: Option<String>,
    /// Hive: Column aliases with optional comments: (c1 COMMENT 'text', c2)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<ViewColumn>,
}

/// Actions for ALTER VIEW
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterViewAction {
    /// Rename the view
    Rename(Box<TableRef>),
    /// Change owner
    OwnerTo(Identifier),
    /// Set schema
    SetSchema(Identifier),
    /// Set authorization (Trino/Presto)
    SetAuthorization(String),
    /// Alter column
    AlterColumn {
        name: Identifier,
        action: Box<AlterColumnAction>,
    },
    /// Redefine view as query (SELECT, UNION, etc.)
    AsSelect(Box<Expression>),
    /// Hive: SET TBLPROPERTIES ('key'='value', ...)
    SetTblproperties(Vec<(String, String)>),
    /// Hive: UNSET TBLPROPERTIES ('key1', 'key2', ...)
    UnsetTblproperties(Vec<String>),
}

impl AlterView {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            actions: Vec::new(),
            algorithm: None,
            definer: None,
            sql_security: None,
            with_option: None,
            columns: Vec::new(),
        }
    }
}

/// ALTER INDEX statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterIndex {
    pub name: Identifier,
    pub table: Option<TableRef>,
    pub actions: Vec<AlterIndexAction>,
}

/// Actions for ALTER INDEX
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterIndexAction {
    /// Rename the index
    Rename(Identifier),
    /// Set tablespace
    SetTablespace(Identifier),
    /// Set visibility (MySQL)
    Visible(bool),
}

impl AlterIndex {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            table: None,
            actions: Vec::new(),
        }
    }
}

/// CREATE SCHEMA statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSchema {
    /// Schema name parts, possibly dot-qualified (e.g. [mydb, hr] for "mydb.hr")
    pub name: Vec<Identifier>,
    pub if_not_exists: bool,
    pub authorization: Option<Identifier>,
    /// CLONE source parts, possibly dot-qualified
    #[serde(default)]
    pub clone_from: Option<Vec<Identifier>>,
    /// AT/BEFORE clause for time travel (Snowflake)
    #[serde(default)]
    pub at_clause: Option<Expression>,
    /// Schema properties like DEFAULT COLLATE
    #[serde(default)]
    pub properties: Vec<Expression>,
    /// Leading comments before the statement
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leading_comments: Vec<String>,
}

impl CreateSchema {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: vec![Identifier::new(name)],
            if_not_exists: false,
            authorization: None,
            clone_from: None,
            at_clause: None,
            properties: Vec::new(),
            leading_comments: Vec::new(),
        }
    }
}

/// DROP SCHEMA statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropSchema {
    pub name: Identifier,
    pub if_exists: bool,
    pub cascade: bool,
    /// RESTRICT, kept apart from the default; left out of the tree when absent
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub restrict: bool,
}

impl DropSchema {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            if_exists: false,
            cascade: false,
            restrict: false,
        }
    }
}

/// DROP NAMESPACE statement (Spark/Databricks - alias for DROP SCHEMA)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropNamespace {
    pub name: Identifier,
    pub if_exists: bool,
    pub cascade: bool,
}

impl DropNamespace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            if_exists: false,
            cascade: false,
        }
    }
}

/// CREATE DATABASE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateDatabase {
    pub name: Identifier,
    pub if_not_exists: bool,
    pub options: Vec<DatabaseOption>,
    /// Snowflake CLONE source
    #[serde(default)]
    pub clone_from: Option<Identifier>,
    /// AT/BEFORE clause for time travel (Snowflake)
    #[serde(default)]
    pub at_clause: Option<Expression>,
}

/// Database option
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DatabaseOption {
    CharacterSet(String),
    Collate(String),
    Owner(Identifier),
    Template(Identifier),
    Encoding(String),
    Location(String),
}

impl CreateDatabase {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            if_not_exists: false,
            options: Vec::new(),
            clone_from: None,
            at_clause: None,
        }
    }
}

/// DROP DATABASE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropDatabase {
    pub name: Identifier,
    pub if_exists: bool,
    /// ClickHouse: SYNC modifier
    #[serde(default)]
    pub sync: bool,
}

impl DropDatabase {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            if_exists: false,
            sync: false,
        }
    }
}

/// CREATE FUNCTION statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateFunction {
    pub name: TableRef,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: Option<DataType>,
    pub body: Option<FunctionBody>,
    pub or_replace: bool,
    /// TSQL: CREATE OR ALTER
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub or_alter: bool,
    pub if_not_exists: bool,
    pub temporary: bool,
    pub language: Option<String>,
    pub deterministic: Option<bool>,
    pub returns_null_on_null_input: Option<bool>,
    pub security: Option<FunctionSecurity>,
    /// Whether parentheses were present in the original syntax
    #[serde(default = "default_true")]
    pub has_parens: bool,
    /// SQL data access characteristic (CONTAINS SQL, READS SQL DATA, etc.)
    #[serde(default)]
    pub sql_data_access: Option<SqlDataAccess>,
    /// TSQL: RETURNS @var TABLE (col_defs) - stores the variable name and column definitions as raw string
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returns_table_body: Option<String>,
    /// True if LANGUAGE clause appears before RETURNS clause
    #[serde(default)]
    pub language_first: bool,
    /// PostgreSQL SET options: SET key = value, SET key FROM CURRENT
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub set_options: Vec<FunctionSetOption>,
    /// True if STRICT was used instead of RETURNS NULL ON NULL INPUT
    #[serde(default)]
    pub strict: bool,
    /// BigQuery: OPTIONS (key=value, ...)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Expression>,
    /// BigQuery: True if this is a TABLE FUNCTION (CREATE TABLE FUNCTION)
    #[serde(default)]
    pub is_table_function: bool,
    /// Original order of function properties (SET, AS, LANGUAGE, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub property_order: Vec<FunctionPropertyKind>,
    /// Hive: USING JAR|FILE|ARCHIVE '...'
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub using_resources: Vec<FunctionUsingResource>,
    /// Databricks: ENVIRONMENT (dependencies = '...', environment_version = '...')
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub environment: Vec<Expression>,
    /// HANDLER 'handler_function' clause (Databricks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handler: Option<String>,
    /// True when the HANDLER clause used Snowflake-style `HANDLER = 'fn'`
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub handler_uses_eq: bool,
    /// Snowflake: RUNTIME_VERSION='3.11'
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_version: Option<String>,
    /// Snowflake: PACKAGES=('pkg1', 'pkg2')
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<String>>,
    /// PARAMETER STYLE clause (e.g., PANDAS for Databricks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter_style: Option<String>,
}

/// A SET option in CREATE FUNCTION (PostgreSQL)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionSetOption {
    pub name: String,
    pub value: FunctionSetValue,
}

/// The value of a SET option
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionSetValue {
    /// SET key = value (use_to = false) or SET key TO value (use_to = true)
    Value { value: String, use_to: bool },
    /// SET key FROM CURRENT
    FromCurrent,
}

/// SQL data access characteristics for functions
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum SqlDataAccess {
    /// NO SQL
    NoSql,
    /// CONTAINS SQL
    ContainsSql,
    /// READS SQL DATA
    ReadsSqlData,
    /// MODIFIES SQL DATA
    ModifiesSqlData,
}

/// Types of properties in CREATE FUNCTION for tracking their original order
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum FunctionPropertyKind {
    /// SET option
    Set,
    /// AS body
    As,
    /// Hive: USING JAR|FILE|ARCHIVE ...
    Using,
    /// LANGUAGE clause
    Language,
    /// IMMUTABLE/VOLATILE/STABLE (determinism)
    Determinism,
    /// CALLED ON NULL INPUT / RETURNS NULL ON NULL INPUT / STRICT
    NullInput,
    /// SECURITY DEFINER/INVOKER
    Security,
    /// SQL data access (CONTAINS SQL, READS SQL DATA, etc.)
    SqlDataAccess,
    /// OPTIONS clause (BigQuery)
    Options,
    /// ENVIRONMENT clause (Databricks)
    Environment,
    /// HANDLER clause (Databricks)
    Handler,
    /// Snowflake: RUNTIME_VERSION='...'
    RuntimeVersion,
    /// Snowflake: PACKAGES=(...)
    Packages,
    /// PARAMETER STYLE clause (Databricks)
    ParameterStyle,
}

/// Hive CREATE FUNCTION resource in a USING clause
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionUsingResource {
    pub kind: String,
    pub uri: String,
}

/// Function parameter
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionParameter {
    pub name: Option<Identifier>,
    pub data_type: DataType,
    pub mode: Option<ParameterMode>,
    pub default: Option<Expression>,
    /// Original text of the mode keyword for case-preserving output (e.g., "inout", "VARIADIC")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode_text: Option<String>,
}

/// Parameter mode (IN, OUT, INOUT, VARIADIC)
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum ParameterMode {
    In,
    Out,
    InOut,
    Variadic,
}

/// Function body
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionBody {
    /// AS $$ ... $$ (dollar-quoted)
    Block(String),
    /// AS 'string' (single-quoted string literal body)
    StringLiteral(String),
    /// AS 'expression'
    Expression(Expression),
    /// EXTERNAL NAME 'library'
    External(String),
    /// RETURN expression
    Return(Expression),
    /// BEGIN ... END block with parsed statements
    Statements(Vec<Expression>),
    /// Dollar-quoted body `$$...$$` or `$tag$...$tag$`: content and optional tag.
    DollarQuoted {
        content: String,
        tag: Option<String>,
    },
    /// BEGIN ... END block preserved as raw text (MySQL procedural bodies)
    RawBlock(String),
}

/// Function security (DEFINER, INVOKER, or NONE)
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum FunctionSecurity {
    Definer,
    Invoker,
    /// StarRocks/MySQL: SECURITY NONE
    None,
}

impl CreateFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            parameters: Vec::new(),
            return_type: None,
            body: None,
            or_replace: false,
            or_alter: false,
            if_not_exists: false,
            temporary: false,
            language: None,
            deterministic: None,
            returns_null_on_null_input: None,
            security: None,
            has_parens: true,
            sql_data_access: None,
            returns_table_body: None,
            language_first: false,
            set_options: Vec::new(),
            strict: false,
            options: Vec::new(),
            is_table_function: false,
            property_order: Vec::new(),
            using_resources: Vec::new(),
            environment: Vec::new(),
            handler: None,
            handler_uses_eq: false,
            runtime_version: None,
            packages: None,
            parameter_style: None,
        }
    }
}

/// DROP FUNCTION statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropFunction {
    pub name: TableRef,
    pub parameters: Option<Vec<DataType>>,
    pub if_exists: bool,
    pub cascade: bool,
}

impl DropFunction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            parameters: None,
            if_exists: false,
            cascade: false,
        }
    }
}

/// CREATE PROCEDURE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateProcedure {
    pub name: TableRef,
    pub parameters: Vec<FunctionParameter>,
    pub body: Option<FunctionBody>,
    pub or_replace: bool,
    /// TSQL: CREATE OR ALTER
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub or_alter: bool,
    pub if_not_exists: bool,
    pub language: Option<String>,
    pub security: Option<FunctionSecurity>,
    /// Return type (Snowflake: RETURNS OBJECT, RETURNS VARCHAR, etc.)
    #[serde(default)]
    pub return_type: Option<DataType>,
    /// Execution context (EXECUTE AS CALLER, EXECUTE AS OWNER)
    #[serde(default)]
    pub execute_as: Option<String>,
    /// TSQL WITH options (ENCRYPTION, RECOMPILE, SCHEMABINDING, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with_options: Vec<String>,
    /// Whether the parameter list had parentheses (false for TSQL procedures without parens)
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub has_parens: bool,
    /// Whether the short form PROC was used (instead of PROCEDURE)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub use_proc_keyword: bool,
    /// Snowflake `RETURNS TABLE (...)` columns, possibly none; `return_type` is then absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returns_table: Option<Vec<StructField>>,
}

impl CreateProcedure {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            parameters: Vec::new(),
            body: None,
            or_replace: false,
            or_alter: false,
            if_not_exists: false,
            language: None,
            security: None,
            return_type: None,
            execute_as: None,
            with_options: Vec::new(),
            has_parens: true,
            use_proc_keyword: false,
            returns_table: None,
        }
    }
}

/// DROP PROCEDURE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropProcedure {
    pub name: TableRef,
    pub parameters: Option<Vec<DataType>>,
    pub if_exists: bool,
    pub cascade: bool,
}

impl DropProcedure {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            parameters: None,
            if_exists: false,
            cascade: false,
        }
    }
}

/// Sequence property tag for ordering
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SeqPropKind {
    Start,
    Increment,
    Minvalue,
    Maxvalue,
    Cache,
    NoCache,
    Cycle,
    NoCycle,
    OwnedBy,
    Order,
    NoOrder,
    Comment,
    /// SHARING=<value> (Oracle)
    Sharing,
    /// KEEP (Oracle)
    Keep,
    /// NOKEEP (Oracle)
    NoKeep,
    /// SCALE [EXTEND|NOEXTEND] (Oracle)
    Scale,
    /// NOSCALE (Oracle)
    NoScale,
    /// SHARD [EXTEND|NOEXTEND] (Oracle)
    Shard,
    /// NOSHARD (Oracle)
    NoShard,
    /// SESSION (Oracle)
    Session,
    /// GLOBAL (Oracle)
    Global,
    /// NOCACHE (single word, Oracle)
    NoCacheWord,
    /// NOCYCLE (single word, Oracle)
    NoCycleWord,
    /// NOMINVALUE (single word, Oracle)
    NoMinvalueWord,
    /// NOMAXVALUE (single word, Oracle)
    NoMaxvalueWord,
}

/// CREATE SYNONYM statement (TSQL)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSynonym {
    /// The synonym name (can be qualified: schema.synonym_name)
    pub name: TableRef,
    /// The target object the synonym refers to
    pub target: TableRef,
}

/// CREATE SEQUENCE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSequence {
    pub name: TableRef,
    pub if_not_exists: bool,
    pub temporary: bool,
    #[serde(default)]
    pub or_replace: bool,
    /// AS <type> clause (e.g., AS SMALLINT, AS BIGINT)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_type: Option<DataType>,
    pub increment: Option<i64>,
    pub minvalue: Option<SequenceBound>,
    pub maxvalue: Option<SequenceBound>,
    pub start: Option<i64>,
    pub cache: Option<i64>,
    pub cycle: bool,
    pub owned_by: Option<TableRef>,
    /// Whether OWNED BY NONE was specified
    #[serde(default)]
    pub owned_by_none: bool,
    /// Snowflake: ORDER or NOORDER (true = ORDER, false = NOORDER, None = not specified)
    #[serde(default)]
    pub order: Option<bool>,
    /// Snowflake: COMMENT = 'value'
    #[serde(default)]
    pub comment: Option<String>,
    /// SHARING=<value> (Oracle)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sharing: Option<String>,
    /// SCALE modifier: Some("EXTEND"), Some("NOEXTEND"), Some("") for plain SCALE
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_modifier: Option<String>,
    /// SHARD modifier: Some("EXTEND"), Some("NOEXTEND"), Some("") for plain SHARD
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shard_modifier: Option<String>,
    /// Tracks the order in which properties appeared in the source
    #[serde(default)]
    pub property_order: Vec<SeqPropKind>,
}

/// Sequence bound (value or NO MINVALUE/NO MAXVALUE)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SequenceBound {
    Value(i64),
    None,
}

impl CreateSequence {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            if_not_exists: false,
            temporary: false,
            or_replace: false,
            as_type: None,
            increment: None,
            minvalue: None,
            maxvalue: None,
            start: None,
            cache: None,
            cycle: false,
            owned_by: None,
            owned_by_none: false,
            order: None,
            comment: None,
            sharing: None,
            scale_modifier: None,
            shard_modifier: None,
            property_order: Vec::new(),
        }
    }
}

/// DROP SEQUENCE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropSequence {
    pub name: TableRef,
    pub if_exists: bool,
    pub cascade: bool,
    /// RESTRICT, kept apart from the default; left out of the tree when absent
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub restrict: bool,
}

impl DropSequence {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            if_exists: false,
            cascade: false,
            restrict: false,
        }
    }
}

/// ALTER SEQUENCE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterSequence {
    pub name: TableRef,
    pub if_exists: bool,
    pub increment: Option<i64>,
    pub minvalue: Option<SequenceBound>,
    pub maxvalue: Option<SequenceBound>,
    pub start: Option<i64>,
    pub restart: Option<Option<i64>>,
    pub cache: Option<i64>,
    pub cycle: Option<bool>,
    pub owned_by: Option<Option<TableRef>>,
}

impl AlterSequence {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            if_exists: false,
            increment: None,
            minvalue: None,
            maxvalue: None,
            start: None,
            restart: None,
            cache: None,
            cycle: None,
            owned_by: None,
        }
    }
}

/// CREATE TRIGGER statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTrigger {
    pub name: Identifier,
    pub table: TableRef,
    pub timing: TriggerTiming,
    pub events: Vec<TriggerEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub for_each: Option<TriggerForEach>,
    pub when: Option<Expression>,
    /// Whether the WHEN clause was parenthesized in the original SQL
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub when_paren: bool,
    pub body: TriggerBody,
    pub or_replace: bool,
    /// TSQL: CREATE OR ALTER
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub or_alter: bool,
    pub constraint: bool,
    pub deferrable: Option<bool>,
    pub initially_deferred: Option<bool>,
    pub referencing: Option<TriggerReferencing>,
}

/// Trigger timing (BEFORE, AFTER, INSTEAD OF)
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum TriggerTiming {
    Before,
    After,
    InsteadOf,
}

/// Trigger event (INSERT, UPDATE, DELETE, TRUNCATE)
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerEvent {
    Insert,
    Update(Option<Vec<Identifier>>),
    Delete,
    Truncate,
}

/// Trigger FOR EACH clause
#[derive(
    sqlquack_syntax_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum TriggerForEach {
    Row,
    Statement,
}

/// Trigger body
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerBody {
    /// EXECUTE FUNCTION/PROCEDURE name(args)
    Execute {
        function: Box<TableRef>,
        args: Vec<Expression>,
    },
    /// BEGIN ... END block
    Block(String),
}

/// Trigger REFERENCING clause
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerReferencing {
    pub old_table: Option<Identifier>,
    pub new_table: Option<Identifier>,
    pub old_row: Option<Identifier>,
    pub new_row: Option<Identifier>,
}

impl CreateTrigger {
    pub fn new(name: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            table: TableRef::new(table),
            timing: TriggerTiming::Before,
            events: Vec::new(),
            for_each: Some(TriggerForEach::Row),
            when: None,
            when_paren: false,
            body: TriggerBody::Execute {
                function: Box::new(TableRef::new("")),
                args: Vec::new(),
            },
            or_replace: false,
            or_alter: false,
            constraint: false,
            deferrable: None,
            initially_deferred: None,
            referencing: None,
        }
    }
}

/// DROP TRIGGER statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropTrigger {
    pub name: Identifier,
    pub table: Option<TableRef>,
    pub if_exists: bool,
    pub cascade: bool,
}

impl DropTrigger {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Identifier::new(name),
            table: None,
            if_exists: false,
            cascade: false,
        }
    }
}

/// CREATE TYPE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateType {
    pub name: TableRef,
    pub definition: TypeDefinition,
    pub if_not_exists: bool,
}

/// Type definition
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeDefinition {
    /// ENUM type: CREATE TYPE name AS ENUM ('val1', 'val2', ...)
    Enum(Vec<String>),
    /// Composite type: CREATE TYPE name AS (field1 type1, field2 type2, ...)
    Composite(Vec<TypeAttribute>),
    /// Range type: CREATE TYPE name AS RANGE (SUBTYPE = type, ...)
    Range {
        subtype: DataType,
        subtype_diff: Option<String>,
        canonical: Option<String>,
    },
    /// Base type (for advanced usage)
    Base {
        input: String,
        output: String,
        internallength: Option<i32>,
    },
    /// Domain type
    Domain {
        base_type: DataType,
        default: Option<Box<Expression>>,
        constraints: Vec<DomainConstraint>,
    },
}

/// Type attribute for composite types
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeAttribute {
    pub name: Identifier,
    pub data_type: DataType,
    pub collate: Option<Identifier>,
}

/// Domain constraint
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainConstraint {
    pub name: Option<Identifier>,
    pub check: Expression,
}

impl CreateType {
    pub fn new_enum(name: impl Into<String>, values: Vec<String>) -> Self {
        Self {
            name: TableRef::new(name),
            definition: TypeDefinition::Enum(values),
            if_not_exists: false,
        }
    }

    pub fn new_composite(name: impl Into<String>, attributes: Vec<TypeAttribute>) -> Self {
        Self {
            name: TableRef::new(name),
            definition: TypeDefinition::Composite(attributes),
            if_not_exists: false,
        }
    }
}

/// DROP TYPE statement
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropType {
    pub name: TableRef,
    pub if_exists: bool,
    pub cascade: bool,
}

impl DropType {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: TableRef::new(name),
            if_exists: false,
            cascade: false,
        }
    }
}

/// Snowflake ALTER of an object kind with no statement of its own, e.g. `ALTER TASK t1 RESUME`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterObject {
    /// The object kind, e.g. "TASK", "TASK GRAPH", "SNAPSHOT SET"
    pub kind: String,
    /// The object name
    pub name: TableRef,
    /// The argument types that identify a function or procedure; a TABLE( ... ) argument is a tuple
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<Expression>,
    /// IF EXISTS clause
    #[serde(default)]
    pub if_exists: bool,
    /// The action's keywords, e.g. "RESUME", "MODIFY RUN", "UNSET TAG"
    pub action: String,
    /// The action's operands in order: names, values, a condition or a statement
    #[serde(default)]
    pub operands: Vec<Expression>,
    /// The properties the action sets or unsets; an unset property has no value
    #[serde(default)]
    pub properties: Vec<ObjectProperty>,
}

/// One property of a Snowflake ALTER action, e.g. `SCHEDULE = '1 MINUTE'` or `TAG t1 = 'v'`.
#[derive(sqlquack_syntax_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectProperty {
    /// The property's keywords, e.g. "SCHEDULE", "CONTACT", "TAG", "EXECUTE AS USER"
    pub kind: String,
    /// The name it qualifies: the tag name, the contact purpose or a TASK_DEFAULTS property
    #[serde(default)]
    pub name: Option<Identifier>,
    /// The value, absent for an unset property
    #[serde(default)]
    pub value: Option<Expression>,
}
