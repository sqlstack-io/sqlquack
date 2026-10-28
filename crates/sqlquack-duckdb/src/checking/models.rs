//! Stand-ins for project functions in DuckDB's check.

use sqlquack_snowflake::typing::models::SnowflakeType;

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
