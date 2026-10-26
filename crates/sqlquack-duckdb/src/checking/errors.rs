//! Why DuckDB rejected printed SQL.

use std::fmt::{self, Display};

/// The step at which DuckDB rejected printed SQL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStage {
    /// DuckDB could not be started.
    Engine,
    /// DuckDB's parser rejected the SQL.
    Parse,
    /// The SQL is not exactly one statement.
    StatementCount,
    /// DuckDB's binder rejected the SQL.
    Bind,
    /// A lambda parameter is read with a dot, so a relation of that alias would capture it.
    CapturedParameter,
}

/// DuckDB rejected printed SQL at `stage`, with DuckDB's message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintedSqlFailure {
    /// Where it failed.
    pub stage: CheckStage,
    /// DuckDB's message.
    pub message: String,
}

impl Display for PrintedSqlFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stage = match self.stage {
            CheckStage::Engine => "DuckDB is unavailable",
            CheckStage::Parse => "DuckDB does not parse it",
            CheckStage::StatementCount => "it is not one statement",
            CheckStage::Bind => "DuckDB does not bind it",
            CheckStage::CapturedParameter => "a relation could capture a lambda parameter",
        };
        write!(formatter, "{stage}: {}", self.message)
    }
}
