//! Data models of expressions statements.

use crate::tokens::models::SourceSpan;
use serde::Deserialize;
use serde::Serialize;

use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::TableRef;
use crate::expressions::queries::models::From;
use crate::expressions::queries::models::Hint;
use crate::expressions::queries::models::Join;
use crate::expressions::queries::models::Limit;
use crate::expressions::queries::models::OrderBy;
use crate::expressions::queries::models::Ordered;
use crate::expressions::queries::models::Over;
use crate::expressions::queries::models::RowFormat;
use crate::expressions::queries::models::Sample;
use crate::expressions::queries::models::Where;
use crate::expressions::queries::models::With;
use crate::expressions::tables::models::OnCluster;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::IntervalUnit;
use crate::expressions::types::models::StructField;

/// Directory insert for INSERT OVERWRITE DIRECTORY (Hive/Spark)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectoryInsert {
    pub local: bool,
    /// The directory; empty when only Databricks' `OPTIONS ( 'path' ... )` names it
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// Databricks: the path as written when it was a raw string or adjacent strings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_written: Option<Literal>,
    pub row_format: Option<RowFormat>,
    /// STORED AS clause (e.g., TEXTFILE, ORC, PARQUET)
    #[serde(default)]
    pub stored_as: Option<String>,
    /// Databricks: the `STORED AS` format was written in backquotes
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ast(skip)]
    pub stored_as_quoted: bool,
    /// Hive: `STORED AS INPUTFORMAT` class
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub input_format: Option<String>,
    /// Databricks: the `INPUTFORMAT` class as written when it was a raw string or adjacent strings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_format_written: Option<Literal>,
    /// Hive: `STORED AS ... OUTPUTFORMAT` class
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub output_format: Option<String>,
    /// Databricks: the `OUTPUTFORMAT` class as written when it was a raw string or adjacent strings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_format_written: Option<Literal>,
    /// Databricks: `USING format`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub using: Option<String>,
    /// Databricks: the `USING` format was written in backquotes
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ast(skip)]
    pub using_quoted: bool,
    /// Databricks: `OPTIONS ( key [=] value, ... )`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<(Expression, Expression)>,
    /// Databricks: the positions in `options` of values written without the optional `=`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ast(skip)]
    pub options_without_equals: Vec<usize>,
}

/// INSERT statement
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Insert {
    pub table: TableRef,
    pub columns: Vec<Identifier>,
    pub values: Vec<Vec<Expression>>,
    pub query: Option<Expression>,
    /// INSERT OVERWRITE for Hive/Spark
    pub overwrite: bool,
    /// PARTITION clause for Hive/Spark
    pub partition: Vec<(Identifier, Option<Expression>)>,
    /// INSERT OVERWRITE DIRECTORY for Hive/Spark
    #[serde(default)]
    pub directory: Option<DirectoryInsert>,
    /// RETURNING clause (PostgreSQL, SQLite)
    #[serde(default)]
    pub returning: Vec<Expression>,
    /// OUTPUT clause (TSQL)
    #[serde(default)]
    pub output: Option<OutputClause>,
    /// ON CONFLICT clause (PostgreSQL, SQLite)
    #[serde(default)]
    pub on_conflict: Option<Box<Expression>>,
    /// Leading comments before the statement
    #[serde(default)]
    pub leading_comments: Vec<String>,
    /// IF EXISTS clause (Hive)
    #[serde(default)]
    pub if_exists: bool,
    /// WITH clause (CTEs)
    #[serde(default)]
    pub with: Option<With>,
    /// INSERT IGNORE (MySQL) - ignore duplicate key errors
    #[serde(default)]
    pub ignore: bool,
    /// Source alias for VALUES clause (MySQL): VALUES (1, 2) AS new_data
    #[serde(default)]
    pub source_alias: Option<Identifier>,
    /// Table alias (PostgreSQL): INSERT INTO table AS t(...)
    #[serde(default)]
    pub alias: Option<Identifier>,
    /// Whether the alias uses explicit AS keyword
    #[serde(default)]
    pub alias_explicit_as: bool,
    /// DEFAULT VALUES (PostgreSQL): INSERT INTO t DEFAULT VALUES
    #[serde(default)]
    pub default_values: bool,
    /// BY NAME modifier (DuckDB): INSERT INTO x BY NAME SELECT ...
    #[serde(default)]
    pub by_name: bool,
    /// SQLite conflict action: INSERT OR ABORT|FAIL|IGNORE|REPLACE|ROLLBACK INTO ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_action: Option<String>,
    /// MySQL/SQLite REPLACE INTO statement (treat like INSERT)
    #[serde(default)]
    pub is_replace: bool,
    /// Oracle-style hint: `INSERT <hint> INTO ...` (for example Oracle APPEND hints)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<Hint>,
    /// REPLACE WHERE clause (Databricks): INSERT INTO a REPLACE WHERE cond VALUES ...
    #[serde(default)]
    pub replace_where: Option<Box<Expression>>,
    /// Source table (Hive/Spark): INSERT OVERWRITE TABLE target TABLE source
    #[serde(default)]
    pub source: Option<Box<Expression>>,
    /// ClickHouse: INSERT INTO FUNCTION func_name(...) - the function call
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_target: Option<Box<Expression>>,
    /// ClickHouse: PARTITION BY expr
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_by: Option<Box<Expression>>,
    /// ClickHouse: SETTINGS key = val, ...
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<Expression>,
    /// PostgreSQL `OVERRIDING USER VALUE` or `OVERRIDING SYSTEM VALUE`: "USER" or "SYSTEM"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overriding: Option<String>,
    /// PostgreSQL `RETURNING WITH (OLD AS o, NEW AS n)`: each `(OLD|NEW, alias)` in order
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub returning_aliases: Vec<Expression>,
    /// PostgreSQL: the column list when a column has a subfield or subscript; `columns` is empty
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_targets: Vec<Expression>,
    /// Databricks: `WITH SCHEMA EVOLUTION` after `INSERT`
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ast(skip)]
    pub schema_evolution: bool,
    /// Databricks: whether `TABLE` was written before the target
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ast(skip)]
    pub table_keyword: bool,
    /// Databricks: `REPLACE ON condition`, the rows the query replaces
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace_on: Option<Box<Expression>>,
    /// Databricks: `REPLACE USING ( columns )`, the columns whose matching rows the query replaces
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replace_using: Vec<Identifier>,
    /// SQLQuack: the INSERT keyword, recorded by a located parse and never serialised.
    #[serde(skip)]
    #[ast(skip)]
    pub keyword_span: crate::tokens::models::SourceSpan,
}

/// OUTPUT clause (TSQL) - used in INSERT, UPDATE, DELETE
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputClause {
    /// Columns/expressions to output
    pub columns: Vec<Expression>,
    /// Optional INTO target table or table variable
    #[serde(default)]
    pub into_table: Option<Expression>,
}

/// UPDATE statement
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Update {
    pub table: TableRef,
    #[serde(default)]
    pub hint: Option<Hint>,
    /// Additional tables for multi-table UPDATE (MySQL syntax)
    #[serde(default)]
    pub extra_tables: Vec<TableRef>,
    /// JOINs attached to the table list (MySQL multi-table syntax)
    #[serde(default)]
    pub table_joins: Vec<Join>,
    pub set: Vec<(Identifier, Expression)>,
    pub from_clause: Option<From>,
    /// JOINs after FROM clause (PostgreSQL, Snowflake, SQL Server syntax)
    #[serde(default)]
    pub from_joins: Vec<Join>,
    pub where_clause: Option<Where>,
    /// RETURNING clause (PostgreSQL, SQLite)
    #[serde(default)]
    pub returning: Vec<Expression>,
    /// OUTPUT clause (TSQL)
    #[serde(default)]
    pub output: Option<OutputClause>,
    /// WITH clause (CTEs)
    #[serde(default)]
    pub with: Option<With>,
    /// Leading comments before the statement
    #[serde(default)]
    pub leading_comments: Vec<String>,
    /// LIMIT clause (MySQL)
    #[serde(default)]
    pub limit: Option<Expression>,
    /// ORDER BY clause (MySQL)
    #[serde(default)]
    pub order_by: Option<OrderBy>,
    /// Whether FROM clause appears before SET (Snowflake syntax)
    #[serde(default)]
    pub from_before_set: bool,
    /// PostgreSQL: the SET list as `(targets, source)` items when `(a, b) = source` is no row; `set` is empty
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assignments: Vec<Expression>,
    /// PostgreSQL `WHERE CURRENT OF cursor`, which stands in place of a WHERE condition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_of: Option<Identifier>,
    /// PostgreSQL `RETURNING WITH (OLD AS o, NEW AS n)`: each `(OLD|NEW, alias)` in order
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub returning_aliases: Vec<Expression>,
    /// SQLQuack: the UPDATE keyword, recorded by a located parse and never serialised.
    #[serde(skip)]
    #[ast(skip)]
    pub keyword_span: crate::tokens::models::SourceSpan,
}

/// DELETE statement
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delete {
    pub table: TableRef,
    #[serde(default)]
    pub hint: Option<Hint>,
    /// ClickHouse: ON CLUSTER clause for distributed DDL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_cluster: Option<OnCluster>,
    /// Optional alias for the table
    pub alias: Option<Identifier>,
    /// Whether the alias was declared with explicit AS keyword
    #[serde(default)]
    pub alias_explicit_as: bool,
    /// PostgreSQL/DuckDB USING clause - additional tables to join
    pub using: Vec<TableRef>,
    pub where_clause: Option<Where>,
    /// OUTPUT clause (TSQL)
    #[serde(default)]
    pub output: Option<OutputClause>,
    /// Leading comments before the statement
    #[serde(default)]
    pub leading_comments: Vec<String>,
    /// WITH clause (CTEs)
    #[serde(default)]
    pub with: Option<With>,
    /// LIMIT clause (MySQL)
    #[serde(default)]
    pub limit: Option<Expression>,
    /// ORDER BY clause (MySQL)
    #[serde(default)]
    pub order_by: Option<OrderBy>,
    /// RETURNING clause (PostgreSQL)
    #[serde(default)]
    pub returning: Vec<Expression>,
    /// Target tables of a MySQL multi-table DELETE.
    #[serde(default)]
    pub tables: Vec<TableRef>,
    /// Whether the targets followed FROM (`DELETE FROM t1, t2 USING ...`) or preceded it.
    #[serde(default)]
    pub tables_from_using: bool,
    /// JOINs in MySQL multi-table DELETE: DELETE t1 FROM t1 LEFT JOIN t2 ...
    #[serde(default)]
    pub joins: Vec<Join>,
    /// FORCE INDEX hint (MySQL): DELETE FROM t FORCE INDEX (idx)
    #[serde(default)]
    pub force_index: Option<String>,
    /// BigQuery-style DELETE without FROM keyword: DELETE table WHERE ...
    #[serde(default)]
    pub no_from: bool,
    /// PostgreSQL `WHERE CURRENT OF cursor`, which stands in place of a WHERE condition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_of: Option<Identifier>,
    /// PostgreSQL `RETURNING WITH (OLD AS o, NEW AS n)`: each `(OLD|NEW, alias)` in order
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub returning_aliases: Vec<Expression>,
    /// PostgreSQL: the USING items when any is not a plain table; `using` is then empty
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub using_items: Vec<Expression>,
    /// PostgreSQL: the joins of the USING list, with `using_items`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub using_joins: Vec<Join>,
    /// ClickHouse: SETTINGS key = val, ...
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<Expression>,
    /// SQLQuack: the DELETE keyword, recorded by a located parse and never serialised.
    #[serde(skip)]
    #[ast(skip)]
    pub keyword_span: crate::tokens::models::SourceSpan,
}

/// COPY statement (Snowflake, PostgreSQL, DuckDB, TSQL)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyStmt {
    /// Target table or query
    pub this: Expression,
    /// True for FROM (loading into table), false for TO (exporting)
    pub kind: bool,
    /// Source/destination file(s) or stage
    pub files: Vec<Expression>,
    /// Copy parameters
    #[serde(default)]
    pub params: Vec<CopyParameter>,
    /// Credentials for external access
    #[serde(default)]
    pub credentials: Option<Box<Credentials>>,
    /// Whether the INTO keyword was used (COPY INTO vs COPY)
    #[serde(default)]
    pub is_into: bool,
    /// Whether parameters are wrapped in WITH (...) syntax
    #[serde(default)]
    pub with_wrapped: bool,
    /// PostgreSQL `COPY ... FROM ... WHERE condition`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_clause: Option<Box<Expression>>,
    /// Snowflake `COPY FILES INTO`, which copies files between stages
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub copy_files: bool,
    /// Databricks: `BY POSITION` after the target, matching source columns by position
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ast(skip)]
    pub by_position: bool,
    /// Databricks: the target's column list
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<Identifier>,
    /// Databricks: the target's own `WITH ( CREDENTIAL ... )`, apart from the source's
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_credentials: Option<Box<Credentials>>,
}

/// COPY parameter (e.g., FILE_FORMAT = CSV or FORMAT PARQUET)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyParameter {
    pub name: String,
    pub value: Option<Expression>,
    pub values: Vec<Expression>,
    /// Whether the parameter used = sign (TSQL: KEY = VALUE vs DuckDB: KEY VALUE)
    #[serde(default)]
    pub eq: bool,
}

/// Credentials for external access (S3, Azure, etc.)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Credentials {
    pub credentials: Vec<(String, String)>,
    pub encryption: Option<String>,
    pub storage: Option<String>,
    /// Databricks: the `CREDENTIAL name` was written in backquotes
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub name_quoted: bool,
    /// Databricks: the positions of the credential options written without `=`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub without_equals: Vec<usize>,
    /// Databricks: the positions of the credential options whose key was written in backquotes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quoted_keys: Vec<usize>,
    /// Databricks: the positions of the credential options whose key was written as a string
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub string_keys: Vec<usize>,
    /// Databricks: the positions of the credential options whose value was not a string, as in `k = 1`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bare_values: Vec<usize>,
    /// Databricks: the credential keys written as a raw string or as adjacent strings, by position, as written
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub written_keys: Vec<(usize, Literal)>,
    /// Databricks: the credential values written as a raw string or as adjacent strings, by position
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub written_values: Vec<(usize, Literal)>,
}

/// Historical data / Time travel (Snowflake) - BEFORE (STATEMENT => ...) or AT (TIMESTAMP => ...)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalData {
    /// The time travel kind: "BEFORE", "AT", or "END" (as an Identifier expression)
    pub this: Box<Expression>,
    /// The time travel type: "STATEMENT", "TIMESTAMP", "OFFSET", "STREAM", or "VERSION"
    pub kind: String,
    /// The expression value (e.g., the statement ID or timestamp)
    pub expression: Box<Expression>,
}

/// SQL Command (COMMIT, ROLLBACK, BEGIN, etc.)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    /// The command text (e.g., "ROLLBACK", "COMMIT", "BEGIN")
    pub this: String,
}

/// PREPARE name [(type, ...)] AS statement.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareStatement {
    /// The prepared statement name.
    pub name: Identifier,
    /// Optional PostgreSQL parameter type list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameter_types: Vec<DataType>,
    /// The statement to execute when the prepared statement is invoked.
    pub statement: Expression,
}

/// T-SQL TRY/CATCH block.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TryCatch {
    /// Statements inside BEGIN TRY ... END TRY.
    #[serde(default)]
    pub try_body: Vec<Expression>,
    /// Statements inside BEGIN CATCH ... END CATCH, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catch_body: Option<Vec<Expression>>,
}

/// TSQL EXEC/EXECUTE [schema.]procedure [@param = value, ...].
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteStatement {
    /// The procedure name (can be qualified: schema.proc_name)
    pub this: Expression,
    /// Optional T-SQL return-status variable (`EXECUTE @status = procedure`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_status: Option<String>,
    /// Named parameters: @param=value pairs
    #[serde(default)]
    pub parameters: Vec<ExecuteParameter>,
    /// Positional prepared statement arguments, used by PostgreSQL EXECUTE name(...).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<Expression>,
    /// Whether this statement represents PostgreSQL-style prepared statement execution.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub prepared: bool,
    /// Trailing clause text (e.g. WITH RESULT SETS ((...)))
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

/// Snowflake `EXECUTE USING POLICY_CONTEXT ( <name> => <value> , ... ) AS <query>`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteUsingPolicyContext {
    /// The named arguments of POLICY_CONTEXT.
    pub arguments: Vec<Expression>,
    pub query: Expression,
}

/// Snowflake `EXECUTE IMMEDIATE <source> [ USING ( <variable> , ... ) ]`, or `EXECUTE IMMEDIATE FROM <location> [ USING ( <name> => <value> , ... ) ] [ DRY_RUN = <bool> ]`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteImmediate {
    /// The text to run: a string, a variable or a `||` concatenation of them, or the location after FROM.
    pub source: Expression,
    /// Whether the source is a file read FROM a location.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from: bool,
    /// USING's bind variables, or its named arguments after FROM.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub using: Vec<Expression>,
    /// DRY_RUN after FROM, when given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}

/// Named parameter in EXEC statement: @name=value
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecuteParameter {
    /// Parameter name (including @)
    pub name: String,
    /// Parameter value
    pub value: Expression,
    /// Whether this is a positional parameter (no = sign)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub positional: bool,
    /// TSQL OUTPUT modifier on parameter
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub output: bool,
    /// SQLQuack: the node's source range, recorded by a parser that sets it.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// MySQL/MariaDB KILL [CONNECTION | QUERY] <id>.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kill {
    /// The target (process ID or connection ID)
    pub this: Expression,
    /// Optional kind: "CONNECTION" or "QUERY"
    pub kind: Option<String>,
}

/// Snowflake CREATE TASK statement
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTask {
    pub or_replace: bool,
    /// Whether this is Snowflake's `CREATE OR ALTER TASK`
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub or_alter: bool,
    pub if_not_exists: bool,
    /// Task name (possibly qualified: db.schema.task), each part with its quoting
    pub name: TableRef,
    /// Raw text of the properties between the name and AS, for a dialect whose properties are not read one by one
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub properties: String,
    /// Snowflake: each property between the name and AS: assignments, TAG, CONTACT and TASK_DEFAULTS pairs, AFTER, EXECUTE AS USER, WHEN, STARTED or SUSPENDED
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_properties: Vec<crate::expressions::schema_objects::models::ObjectProperty>,
    /// The SQL statement body after AS; absent for `CLONE source`
    pub body: Option<Expression>,
    /// Source task of `CREATE TASK name CLONE source`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clone_source: Option<TableRef>,
    /// Whether this is a Snowflake `CREATE TASK GRAPH`, which has properties and no body
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub graph: bool,
    /// Snowflake DCM: written as `DEFINE` instead of `CREATE`
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub define: bool,
}

/// Raw/unparsed SQL
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Raw {
    pub sql: String,
}

/// TRUNCATE TABLE statement
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Truncate {
    /// Target of TRUNCATE (TABLE vs DATABASE)
    #[serde(default)]
    pub target: TruncateTarget,
    /// IF EXISTS clause
    #[serde(default)]
    pub if_exists: bool,
    pub table: TableRef,
    /// ClickHouse: ON CLUSTER clause for distributed DDL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_cluster: Option<OnCluster>,
    pub cascade: bool,
    /// Additional tables for multi-table TRUNCATE
    #[serde(default)]
    pub extra_tables: Vec<TruncateTableEntry>,
    /// RESTART IDENTITY or CONTINUE IDENTITY
    #[serde(default)]
    pub identity: Option<TruncateIdentity>,
    /// RESTRICT option (alternative to CASCADE)
    #[serde(default)]
    pub restrict: bool,
    /// Hive PARTITION clause: PARTITION(key=value, ...)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<Box<Expression>>,
    /// Snowflake `ERROR_TABLE ( <table> )`: the table's error table; left out of the tree when false
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub error_table: bool,
}

/// A table entry in a TRUNCATE statement, with optional ONLY modifier and * suffix
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TruncateTableEntry {
    pub table: TableRef,
    /// Whether the table has a * suffix (inherit children)
    #[serde(default)]
    pub star: bool,
}

/// TRUNCATE target type
#[derive(
    sqlquack_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default,
)]
pub enum TruncateTarget {
    #[default]
    Table,
    Database,
}

/// TRUNCATE identity option
#[derive(sqlquack_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TruncateIdentity {
    Restart,
    Continue,
}

impl Truncate {
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            target: TruncateTarget::Table,
            if_exists: false,
            table: TableRef::new(table),
            on_cluster: None,
            cascade: false,
            extra_tables: Vec::new(),
            identity: None,
            restrict: false,
            partition: None,
            error_table: false,
        }
    }
}

/// USE statement (USE database, USE ROLE, USE WAREHOUSE, USE CATALOG, USE SCHEMA)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Use {
    /// The kind of object (DATABASE, SCHEMA, ROLE, WAREHOUSE, CATALOG, or None for default)
    pub kind: Option<UseKind>,
    /// The name of the object
    pub this: Identifier,
    /// Snowflake's secondary roles one by one, quoting kept; empty for ALL, NONE and other kinds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Identifier>,
    /// Snowflake's qualified name part by part, quoting kept; empty for a one-part name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<Identifier>,
    /// Databricks: whether the name was written as a string literal
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub string_name: bool,
    /// Databricks: the string name as written when it was a raw string or adjacent strings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_written: Option<Literal>,
    /// Databricks: NAMESPACE written for DATABASE, or SET for `SET CATALOG` instead of `USE CATALOG`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub keyword: Option<String>,
}

/// Kind of USE statement
#[derive(sqlquack_derive::AstNode, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseKind {
    Database,
    Schema,
    Role,
    Warehouse,
    Catalog,
    /// Snowflake: USE SECONDARY ROLES with a list of roles in `roles`
    SecondaryRoles,
    /// Snowflake: USE SECONDARY ROLES ALL
    SecondaryRolesAll,
    /// Snowflake: USE SECONDARY ROLES NONE
    SecondaryRolesNone,
}

/// SET variable statement
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetStatement {
    /// The items being set
    pub items: Vec<SetItem>,
}

/// Snowflake `UNSET name` or `UNSET ( name [ , ... ] )`: drops session variables.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnsetStatement {
    /// The variables, in order.
    pub names: Vec<Identifier>,
    /// Whether the names were written in parentheses.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub parenthesized: bool,
}

/// `RESET`: returns session settings to their defaults, in any dialect that has it.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResetStatement {
    /// The scope written before the target, such as `SESSION` or `GLOBAL`, upper-cased.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// The setting reset; none for `RESET ALL` and for a bare `RESET`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Expression>,
    /// Whether `ALL` was written as the target.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub all: bool,
}

/// A single SET item (variable assignment)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetItem {
    /// The variable name
    pub name: Expression,
    /// The value to set
    pub value: Expression,
    /// Kind: None for plain SET, Some("GLOBAL") for SET GLOBAL, etc.
    pub kind: Option<String>,
    /// Whether the SET item was parsed without an = sign (TSQL: SET KEY VALUE)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_equals: bool,
}

/// CACHE TABLE statement (Spark)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cache {
    /// The table to cache
    pub table: Identifier,
    /// LAZY keyword - defer caching until first use
    pub lazy: bool,
    /// Optional OPTIONS clause (key-value pairs)
    pub options: Vec<(Expression, Expression)>,
    /// Databricks: the positions in `options` of values written without the optional `=`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ast(skip)]
    pub options_without_equals: Vec<usize>,
    /// Optional AS clause with query
    pub query: Option<Expression>,
    /// Whether AS was written before the query
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub query_as: bool,
}

/// UNCACHE TABLE statement (Spark)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Uncache {
    /// The table to uncache
    pub table: Identifier,
    /// IF EXISTS clause
    pub if_exists: bool,
}

/// LOAD DATA statement (Hive)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadData {
    /// LOCAL keyword - load from local filesystem
    pub local: bool,
    /// The path to load data from (INPATH value)
    pub inpath: String,
    /// Databricks: the path as written when it was a raw string or adjacent strings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inpath_written: Option<Literal>,
    /// Whether to overwrite existing data
    pub overwrite: bool,
    /// The target table
    pub table: Expression,
    /// Optional PARTITION clause with key-value pairs
    pub partition: Vec<(Identifier, Expression)>,
    /// Optional INPUTFORMAT clause
    pub input_format: Option<String>,
    /// Optional SERDE clause
    pub serde: Option<String>,
}

/// PRAGMA statement (SQLite)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pragma {
    /// Optional schema prefix (e.g., "schema" in "schema.pragma_name")
    pub schema: Option<Identifier>,
    /// The pragma name
    pub name: Identifier,
    /// Optional value for assignment (PRAGMA name = value)
    pub value: Option<Expression>,
    /// Optional arguments for function-style pragmas (PRAGMA name(arg))
    pub args: Vec<Expression>,
    /// Whether this pragma should be generated using assignment syntax.
    #[serde(default)]
    pub use_assignment_syntax: bool,
}

/// A PostgreSQL utility or session statement with no statement of its own, e.g. `VACUUM (FULL) t` or `LISTEN c`.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtilityStatement {
    /// The statement's keywords, e.g. "VACUUM", "LISTEN", "DECLARE CURSOR", "SET CONSTRAINTS"
    pub kind: String,
    /// The statement's options in order, e.g. `(FULL, true)` or `(SCROLL)`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Expression>,
    /// The statement's operands in order: names, values, settings or a statement
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operands: Vec<Expression>,
}

/// COMMENT ON statement
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    /// The object being commented on
    pub this: Expression,
    /// The object kind (COLUMN, TABLE, DATABASE, etc.)
    pub kind: String,
    /// The comment text expression
    pub expression: Expression,
    /// IF EXISTS clause
    pub exists: bool,
    /// MATERIALIZED keyword
    pub materialized: bool,
}

/// DESCRIBE statement - shows table structure or query plan
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Describe {
    /// The target to describe (table name or query)
    pub target: Expression,
    /// EXTENDED format
    pub extended: bool,
    /// FORMATTED format
    pub formatted: bool,
    /// Object kind (e.g., "SEMANTIC VIEW", "TABLE", etc.)
    #[serde(default)]
    pub kind: Option<String>,
    /// Properties like type=stage
    #[serde(default)]
    pub properties: Vec<(String, String)>,
    /// Style keyword (e.g., "ANALYZE", "HISTORY")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Partition specification for DESCRIBE PARTITION
    #[serde(default)]
    pub partition: Option<Box<Expression>>,
    /// Leading comments before the statement
    #[serde(default)]
    pub leading_comments: Vec<String>,
    /// AS JSON suffix (Databricks)
    #[serde(default)]
    pub as_json: bool,
    /// Parenthesized parameter types for DESCRIBE PROCEDURE/FUNCTION (e.g., INT, VARCHAR)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<DataType>,
    /// Whether the short DESC was written for DESCRIBE
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub short_form: bool,
}

impl Describe {
    pub fn new(target: Expression) -> Self {
        Self {
            target,
            extended: false,
            formatted: false,
            kind: None,
            properties: Vec::new(),
            style: None,
            partition: None,
            leading_comments: Vec::new(),
            as_json: false,
            params: Vec::new(),
            short_form: false,
        }
    }
}

/// SHOW statement - displays database objects
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Show {
    /// The thing to show (DATABASES, TABLES, SCHEMAS, etc.)
    pub this: String,
    /// Whether TERSE was specified
    #[serde(default)]
    pub terse: bool,
    /// Whether HISTORY was specified
    #[serde(default)]
    pub history: bool,
    /// LIKE pattern
    pub like: Option<Expression>,
    /// IN scope kind (ACCOUNT, DATABASE, SCHEMA, TABLE)
    pub scope_kind: Option<String>,
    /// IN scope object
    pub scope: Option<Expression>,
    /// STARTS WITH pattern
    pub starts_with: Option<Expression>,
    /// LIMIT clause
    pub limit: Option<Box<Limit>>,
    /// FROM clause (for specific object)
    pub from: Option<Expression>,
    /// WHERE clause (MySQL: SHOW STATUS WHERE ...)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub where_clause: Option<Expression>,
    /// FOR target (MySQL: SHOW GRANTS FOR user, SHOW PROFILE ... FOR QUERY n)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub for_target: Option<Expression>,
    /// Second FROM clause (MySQL: SHOW COLUMNS FROM tbl FROM db)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db: Option<Expression>,
    /// Target identifier (MySQL: engine name in SHOW ENGINE, table in SHOW COLUMNS FROM)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Expression>,
    /// MUTEX flag for SHOW ENGINE (true=MUTEX, false=STATUS, None=neither)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mutex: Option<bool>,
    /// WITH PRIVILEGES clause (Snowflake: SHOW ... WITH PRIVILEGES USAGE, MODIFY)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub privileges: Vec<String>,
    /// Snowflake SHOW TASKS ... ROOT ONLY, which lists only root tasks
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub root_only: bool,
    /// Snowflake SHOW AVAILABLE LISTINGS filters set to TRUE, such as IS_IMPORTED, in source order
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub listing_filters: Vec<String>,
    /// Snowflake SHOW SERVICES EXCLUDE JOBS
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclude_jobs: bool,
    /// Snowflake SHOW SERVICES OF TYPE <type> [ , ... ], upper case in source order
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub service_types: Vec<String>,
    /// The FROM or IN written before `scope` or `db`, when either was
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_keyword: Option<String>,
    /// The FROM or IN written before `target`, when either was
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_keyword: Option<String>,
    /// Whether LIKE was written before the `like` pattern
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub like_keyword: bool,
}

impl Show {
    pub fn new(this: impl Into<String>) -> Self {
        Self {
            this: this.into(),
            terse: false,
            history: false,
            like: None,
            scope_kind: None,
            scope: None,
            starts_with: None,
            limit: None,
            from: None,
            where_clause: None,
            for_target: None,
            db: None,
            target: None,
            mutex: None,
            privileges: Vec::new(),
            root_only: false,
            listing_filters: Vec::new(),
            exclude_jobs: false,
            service_types: Vec::new(),
            scope_keyword: None,
            target_keyword: None,
            like_keyword: false,
        }
    }
}

/// Explicit parentheses, kept so `(a + b) * c` round-trips.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paren {
    /// The inner expression wrapped by parentheses.
    pub this: Expression,
    #[serde(default)]
    pub trailing_comments: Vec<String>,
    /// SQLQuack: the node's source range, recorded by a located parse.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// Expression annotated with trailing comments (for round-trip preservation)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotated {
    pub this: Expression,
    pub trailing_comments: Vec<String>,
    /// Type asserted by a lowered [`TYPED_EXPRESSION_FUNCTION`] call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub type_assertion: Option<DataType>,
    /// Expression whose type a lowered [`TYPED_LIKE_EXPRESSION_FUNCTION`] call asserts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub type_like: Option<Box<Expression>>,
}

/// Refresh
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Refresh {
    pub this: Box<Expression>,
    pub kind: String,
}

/// LockingStatement
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LockingStatement {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// SequenceProperties
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceProperties {
    #[serde(default)]
    pub increment: Option<Box<Expression>>,
    #[serde(default)]
    pub minvalue: Option<Box<Expression>>,
    #[serde(default)]
    pub maxvalue: Option<Box<Expression>>,
    #[serde(default)]
    pub cache: Option<Box<Expression>>,
    #[serde(default)]
    pub start: Option<Box<Expression>>,
    #[serde(default)]
    pub owned: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
}

/// TruncateTable
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TruncateTable {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub is_database: Option<Box<Expression>>,
    #[serde(default)]
    pub exists: bool,
    #[serde(default)]
    pub only: Option<Box<Expression>>,
    #[serde(default)]
    pub cluster: Option<Box<Expression>>,
    #[serde(default)]
    pub identity: Option<Box<Expression>>,
    #[serde(default)]
    pub option: Option<Box<Expression>>,
    #[serde(default)]
    pub partition: Option<Box<Expression>>,
}

/// Clone
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clone {
    pub this: Box<Expression>,
    #[serde(default)]
    pub shallow: Option<Box<Expression>>,
    #[serde(default)]
    pub copy: Option<Box<Expression>>,
}

/// Attach
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attach {
    pub this: Box<Expression>,
    #[serde(default)]
    pub exists: bool,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Detach
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detach {
    pub this: Box<Expression>,
    #[serde(default)]
    pub exists: bool,
}

/// Install
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Install {
    pub this: Box<Expression>,
    #[serde(default)]
    pub from_: Option<Box<Expression>>,
    #[serde(default)]
    pub force: Option<Box<Expression>>,
}

/// Summarize
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summarize {
    pub this: Box<Expression>,
    #[serde(default)]
    pub table: Option<Box<Expression>>,
}

/// Declare
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Declare {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub replace: bool,
    /// Databricks: VAR or VARIABLE as written after DECLARE [OR REPLACE]; none when omitted
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub keyword: Option<String>,
}

/// DeclareItem
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclareItem {
    pub this: Box<Expression>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub default: Option<Box<Expression>>,
    #[serde(default)]
    pub has_as: bool,
    /// Databricks: whether `=` was written for DEFAULT
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub equals: bool,
    /// BigQuery: additional variable names in multi-variable DECLARE (DECLARE X, Y, Z INT64)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_names: Vec<Expression>,
    /// SQLQuack: the node's source range, recorded by a parser that sets it.
    #[serde(default, skip_serializing_if = "SourceSpan::is_absent")]
    #[ast(skip)]
    pub span: SourceSpan,
}

/// Set
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Set {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub unset: Option<Box<Expression>>,
    #[serde(default)]
    pub tag: Option<Box<Expression>>,
}

/// Heredoc
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heredoc {
    pub this: Box<Expression>,
    #[serde(default)]
    pub tag: Option<Box<Expression>>,
}

/// QueryBand
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryBand {
    pub this: Box<Expression>,
    #[serde(default)]
    pub scope: Option<Box<Expression>>,
    #[serde(default)]
    pub update: Option<Box<Expression>>,
}

/// UserDefinedFunction
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserDefinedFunction {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub wrapped: Option<Box<Expression>>,
}

/// RecursiveWithSearch
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecursiveWithSearch {
    pub kind: String,
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub using: Option<Box<Expression>>,
}

/// ProjectionDef
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectionDef {
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
}

/// TableAlias
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableAlias {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub columns: Vec<Expression>,
}

/// Get
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Get {
    pub this: Box<Expression>,
    #[serde(default)]
    pub target: Option<Box<Expression>>,
    #[serde(default)]
    pub properties: Vec<Expression>,
}

/// SetOperation
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetOperation {
    #[serde(default)]
    pub with_: Option<Box<Expression>>,
    pub this: Box<Expression>,
    pub expression: Box<Expression>,
    #[serde(default)]
    pub distinct: bool,
    #[serde(default)]
    pub by_name: Option<Box<Expression>>,
    #[serde(default)]
    pub side: Option<Box<Expression>>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub on: Option<Box<Expression>>,
}

/// Var - Simple variable reference (for SQL variables, keywords as values)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Var {
    pub this: String,
}

/// Variadic - represents VARIADIC prefix on function arguments (PostgreSQL)
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variadic {
    pub this: Box<Expression>,
}

/// Version
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Version {
    pub this: Box<Expression>,
    pub kind: String,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub for_keyword: bool,
}

/// Schema
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// Lock
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lock {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_locked: bool,
    #[serde(default)]
    pub update: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub wait: Option<Box<Expression>>,
    #[serde(default)]
    pub key: Option<Box<Expression>>,
}

/// TABLESAMPLE after a non-table source, such as a subquery or a function.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableSample {
    /// The expression being sampled (subquery, function, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub this: Option<Box<Expression>>,
    /// The sample specification
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<Box<Sample>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub bucket_numerator: Option<Box<Expression>>,
    #[serde(default)]
    pub bucket_denominator: Option<Box<Expression>>,
    #[serde(default)]
    pub bucket_field: Option<Box<Expression>>,
    #[serde(default)]
    pub percent: Option<Box<Expression>>,
    #[serde(default)]
    pub rows: Option<Box<Expression>>,
    #[serde(default)]
    pub size: Option<i64>,
    #[serde(default)]
    pub seed: Option<Box<Expression>>,
}

/// Tags are used for generating arbitrary sql like SELECT <span>x</span>.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub prefix: Option<Box<Expression>>,
    #[serde(default)]
    pub postfix: Option<Box<Expression>>,
}

/// UnpivotColumns
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnpivotColumns {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// SessionParameter
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionParameter {
    pub this: Box<Expression>,
    #[serde(default)]
    pub kind: Option<String>,
}

/// PseudoType
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PseudoType {
    pub this: Box<Expression>,
}

/// ObjectIdentifier
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectIdentifier {
    pub this: Box<Expression>,
}

/// Transaction
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub modes: Option<Box<Expression>>,
    #[serde(default)]
    pub mark: Option<Box<Expression>>,
}

/// Commit
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Commit {
    #[serde(default)]
    pub chain: Option<Box<Expression>>,
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub durability: Option<Box<Expression>>,
}

/// Rollback
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rollback {
    #[serde(default)]
    pub savepoint: Option<Box<Expression>>,
    #[serde(default)]
    pub this: Option<Box<Expression>>,
}

/// AlterSession
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterSession {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub unset: Option<Box<Expression>>,
}

/// Analyze
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Analyze {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub options: Vec<Expression>,
    #[serde(default)]
    pub mode: Option<Box<Expression>>,
    #[serde(default)]
    pub partition: Option<Box<Expression>>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub properties: Vec<Expression>,
    /// Column list for ANALYZE tbl(col1, col2) syntax (PostgreSQL)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<String>,
}

/// AnalyzeStatistics
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeStatistics {
    pub kind: String,
    #[serde(default)]
    pub option: Option<Box<Expression>>,
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// AnalyzeHistogram
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeHistogram {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
    #[serde(default)]
    pub update_options: Option<Box<Expression>>,
}

/// AnalyzeSample
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeSample {
    pub kind: String,
    #[serde(default)]
    pub sample: Option<Box<Expression>>,
}

/// AnalyzeListChainedRows
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeListChainedRows {
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// AnalyzeDelete
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeDelete {
    #[serde(default)]
    pub kind: Option<String>,
}

/// AnalyzeWith
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeWith {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// AnalyzeValidate
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeValidate {
    pub kind: String,
    #[serde(default)]
    pub this: Option<Box<Expression>>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// AddPartition
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddPartition {
    pub this: Box<Expression>,
    #[serde(default)]
    pub exists: bool,
    #[serde(default)]
    pub location: Option<Box<Expression>>,
}

/// AttachOption
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttachOption {
    pub this: Box<Expression>,
    #[serde(default)]
    pub expression: Option<Box<Expression>>,
}

/// DropPartition
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropPartition {
    #[serde(default)]
    pub expressions: Vec<Expression>,
    #[serde(default)]
    pub exists: bool,
}

/// ReplacePartition
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplacePartition {
    pub expression: Box<Expression>,
    #[serde(default)]
    pub source: Option<Box<Expression>>,
}

/// Merge
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Merge {
    pub this: Box<Expression>,
    pub using: Box<Expression>,
    #[serde(default)]
    pub on: Option<Box<Expression>>,
    #[serde(default)]
    pub using_cond: Option<Box<Expression>>,
    #[serde(default)]
    pub whens: Option<Box<Expression>>,
    #[serde(default)]
    pub with_: Option<Box<Expression>>,
    #[serde(default)]
    pub returning: Option<Box<Expression>>,
    /// PostgreSQL `RETURNING WITH (OLD AS o, NEW AS n)`: each `(OLD|NEW, alias)` in order
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub returning_aliases: Vec<Expression>,
    /// Databricks: `WITH SCHEMA EVOLUTION` after `MERGE`
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[ast(skip)]
    pub schema_evolution: bool,
}

/// When
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct When {
    #[serde(default)]
    pub matched: Option<Box<Expression>>,
    #[serde(default)]
    pub source: Option<Box<Expression>>,
    #[serde(default)]
    pub condition: Option<Box<Expression>>,
    pub then: Box<Expression>,
    /// Databricks: the NOT of `WHEN NOT MATCHED` as written, when another documented spelling shares this node
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ast(skip)]
    pub not_written_as: Option<crate::expressions::operators::models::OperatorSpelling>,
}

/// Wraps around one or more WHEN [NOT] MATCHED [...] clauses.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Whens {
    #[serde(default)]
    pub expressions: Vec<Expression>,
}

/// NextValueFor
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NextValueFor {
    pub this: Box<Expression>,
    #[serde(default)]
    pub order: Option<Box<Expression>>,
}

/// Native Vertica syntax, retaining semantics through serialization and traversal.
#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VerticaExpression {
    PhysicalTable {
        this: Expression,
        physical: VerticaPhysical,
    },
    FlexTable {
        this: Expression,
        physical: VerticaPhysical,
    },
    Projection {
        name: TableRef,
        columns: Vec<VerticaProjectionColumn>,
        query: Expression,
        physical: VerticaPhysical,
    },
    Copy {
        table: TableRef,
        columns: Vec<Identifier>,
        local: bool,
        sources: Vec<Expression>,
        parser: Option<Box<VerticaParserCall>>,
        options: Vec<VerticaParameter>,
    },
    ExportParquet {
        options: Vec<VerticaParameter>,
        over: Option<Over>,
        query: Expression,
    },
    Interpolate {
        left: Expression,
        right: Expression,
        previous: bool,
    },
    /// Zero-based array access retains source bounds semantics from parsing.
    ArrayAccess {
        this: Expression,
        indices: Vec<Expression>,
    },
    ArraySlice {
        this: Expression,
        start: Option<Expression>,
        end: Option<Expression>,
    },
    BoundaryDateDiff {
        start: Expression,
        end: Expression,
        unit: IntervalUnit,
    },
    UsingParameters {
        this: Expression,
        parameters: Vec<VerticaParameter>,
    },
    SafeCast {
        this: Expression,
        to: DataType,
    },
    Set {
        values: Vec<Expression>,
    },
    Interval {
        value: Expression,
        precision: Option<u32>,
        year_month: bool,
        unit: Option<String>,
        end_unit: Option<String>,
    },
    /// Hexadecimal byte value, distinct from PostgreSQL's bit strings.
    Binary {
        hex: String,
    },
    Historical {
        query: Expression,
        point: VerticaHistoricalPoint,
    },
    PartitionBest,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerticaParameter {
    pub name: Identifier,
    pub value: Expression,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum VerticaHistoricalPoint {
    Latest,
    Epoch(Expression),
    Time(Expression),
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VerticaDataType {
    Array {
        element_type: DataType,
        bound: Option<VerticaCollectionBound>,
    },
    Set {
        element_type: DataType,
        bound: Option<VerticaCollectionBound>,
    },
    Row {
        fields: Vec<StructField>,
    },
    LongVarBinary {
        length: Option<u32>,
    },
    Interval {
        precision: Option<u32>,
        year_month: bool,
        unit: Option<String>,
        end_unit: Option<String>,
    },
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum VerticaCollectionBound {
    Elements(u32),
    Bytes(u32),
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VerticaSelectExtensions {
    pub timeseries: Option<VerticaTimeseries>,
    pub match_clause: Option<VerticaMatch>,
    pub limit_over: Option<Over>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerticaTimeseries {
    pub alias: Identifier,
    pub interval: Expression,
    pub over: Over,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerticaMatch {
    pub partition_by: Vec<Expression>,
    pub order_by: Vec<Ordered>,
    pub definitions: Vec<VerticaParameter>,
    pub name: Identifier,
    pub pattern: VerticaPattern,
    /// None = default; true = FIRST EVENT; false = ALL EVENTS.
    pub first_event: Option<bool>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum VerticaPattern {
    Event(Identifier),
    Sequence(Vec<VerticaPattern>),
    Alternative(Vec<VerticaPattern>),
    Group(Box<VerticaPattern>),
    Repeat {
        pattern: Box<VerticaPattern>,
        quantifier: String,
    },
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VerticaPhysical {
    pub order_by: Vec<Ordered>,
    pub segmentation: Option<VerticaSegmentation>,
    pub ksafe: Option<VerticaKsafe>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum VerticaKsafe {
    Default,
    Level(u32),
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VerticaSegmentation {
    Segmented {
        expression: Expression,
        offset: Option<u32>,
    },
    Unsegmented {
        node: Option<Identifier>,
    },
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerticaProjectionColumn {
    pub name: Identifier,
    pub encoding: Option<Identifier>,
}

#[derive(sqlquack_derive::AstNode, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerticaParserCall {
    pub name: TableRef,
    pub parameters: Vec<VerticaParameter>,
}
