//! The query that reads DuckDB's `json_serialize_sql` output, and the holding types of tables.

/// DuckDB's parse error and message, statement count, and base-table and function names (JSON).
pub const SERIALISED_SUMMARY: &str = "\
    SELECT coalesce((tree ->> '$.error')::BOOLEAN, false), \
           tree ->> '$.error_message', \
           coalesce(json_array_length(tree -> '$.statements'), 0), \
           to_json(coalesce(list_distinct(json_extract_string(tree, '$..table_name')), []))::VARCHAR, \
           to_json(coalesce(list_distinct(json_extract_string(tree, '$..function_name')), []))::VARCHAR \
    FROM (SELECT json_serialize_sql(?::VARCHAR) AS tree)";
/// The DuckDB type semi-structured Snowflake values are held in (decision 3: JSON, not VARIANT).
pub const SEMI_STRUCTURED_HOLDING_TYPE: &str = "JSON";

/// The keyword opening a DuckDB lambda in emitted SQL.
pub const LAMBDA_KEYWORD: &str = "lambda ";
