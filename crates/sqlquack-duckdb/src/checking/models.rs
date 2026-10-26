//! What DuckDB's serialiser reports about printed SQL, and stand-ins for project functions.

use sqlquack_snowflake::typing::models::SnowflakeType;

/// DuckDB's parse result for printed SQL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerialisedSummary {
    /// Whether DuckDB's parser failed.
    pub error: bool,
    /// Its message when it failed.
    pub message: Option<String>,
    /// How many statements it read.
    pub count: i64,
    /// The distinct base-table names read, CTE references included, as a JSON array.
    pub tables: String,
    /// The distinct function names called, as a JSON array.
    pub functions: String,
}

/// What a project function returns, for the stand-in the check defines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StubReturns {
    /// One value of this type.
    Scalar(SnowflakeType),
    /// Rows of these columns.
    Table(Vec<(String, SnowflakeType)>),
}

/// A project function the check stands in for with a DuckDB macro of the same shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionStub {
    /// How many arguments it takes.
    pub arguments: usize,
    /// What it returns.
    pub returns: StubReturns,
}
