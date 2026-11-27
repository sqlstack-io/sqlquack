//! How the Databricks recording tool writes its journals.

/// The client every Databricks journal was recorded through.
pub const DATABRICKS_CONNECTOR: &str = "Databricks SQL Statement Execution API 2.0";
/// What names the importer in the results it writes.
pub const IMPORTER: &str = "sqlquack import-databricks";
/// The journal state of a statement that returned rows.
pub const SUCCEEDED: &str = "SUCCEEDED";
/// The journal state of a statement that raised an error.
pub const FAILED: &str = "FAILED";
/// What precedes the SQLSTATE in a Databricks error message.
pub const SQLSTATE_MARKER: &str = "SQLSTATE: ";
/// The length of a SQLSTATE.
pub const SQLSTATE_LENGTH: usize = 5;
/// The length of a SQLSTATE's class.
pub const SQLSTATE_CLASS_LENGTH: usize = 2;
