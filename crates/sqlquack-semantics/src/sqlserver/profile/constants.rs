//! The values SQL Server's profile and lineage rules read.

/// What SQL Server stores a timestamp written without its kind as; its TIMESTAMP is rowversion.
pub(crate) const SQLSERVER_TIMESTAMP_TYPE: &str = "DATETIME2";
