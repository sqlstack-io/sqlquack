#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Generic,
    Snowflake,
    BigQuery,
    Databricks,
    DuckDb,
}
