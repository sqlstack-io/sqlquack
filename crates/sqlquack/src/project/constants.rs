//! Project folder names, file kinds and the SQLBuild header and decorator keys SQLQuack reads.

/// The project folder of source YAML files, beside the policy file.
pub const SOURCES_FOLDER: &str = "sources";
/// The project folder of function files, beside the policy file.
pub const FUNCTIONS_FOLDER: &str = "functions";
/// The project folder of user overrides, beside the policy file.
pub const OVERRIDES_FOLDER: &str = "overrides";
/// The functions subfolder of SQL function files.
pub const SQL_FUNCTIONS_FOLDER: &str = "sql";
/// The functions subfolder of Python function files.
pub const PYTHON_FUNCTIONS_FOLDER: &str = "python";
/// The extensions of source files.
pub const SOURCE_EXTENSIONS: &[&str] = &["yml", "yaml"];
/// The extension of Python function files.
pub const PYTHON_EXTENSION: &str = "py";
/// SQL function header properties SQLBuild reads that a signature does not need.
pub const SQL_FUNCTION_KEYS: &[&str] = &["description", "tags", "database", "schema"];
/// Keys SQLQuack reads beyond SQLBuild's, in a SQL function header or `@udf(...)`: how calls behave.
pub const FUNCTION_TRAIT_KEYS: &[&str] = &["volatility", "null_input", "vectorized"];
/// The word that starts a table function's `returns table (...)`.
pub const TABLE_RETURNS: &str = "table";
/// How a Python function file's decorator starts.
pub const UDF_DECORATOR: &str = "@udf(";
/// The modules a Python function file may import `udf` from: SQLBuild's and SQLQuack's.
pub const UDF_IMPORTS: &[&str] = &["sqlbuild.functions", "sqlbuild", "sqlquack"];
/// The keywords SQLBuild's `@udf(...)` accepts.
pub const UDF_KEYS: &[&str] = &[
    "description",
    "arguments",
    "returns",
    "runtime_version",
    "entry_point",
    "packages",
    "database",
    "schema",
    "tags",
];
/// The extension of SQL function files.
pub const SQL_EXTENSION: &str = "sql";
/// How a clash names a declared table.
pub const TABLE_KIND: &str = "table";
/// How a clash names a declared function.
pub const FUNCTION_KIND: &str = "function";
/// How a clash names the schema a caller passes in memory, as the Python API's `schema` option.
pub const GIVEN_SCHEMA: &str = "the schema option";
