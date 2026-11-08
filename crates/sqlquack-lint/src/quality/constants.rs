use crate::lint::models::LintRuleMetadata;
use sqlquack_syntax::tokens::models::TokenType;

pub(crate) const UNUSED_CTE_OUTPUT: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN042",
    message: "CTE output column is never read later in the query",
    remediation: "Remove the column from the CTE output, or read it in a later CTE or the final query.",
};
pub(crate) const RANKING_SORT_CAP: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN043",
    message: "Ranking window sorts by more expressions than max_ranking_order_by",
    remediation: "Order by the one or two columns that decide which row wins, then a unique row key as the final tie-breaker.",
};
pub(crate) const LONG_LITERAL: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN044",
    message: "String literal is longer than max_literal_length",
    remediation: "Shorten the literal, or move the value into a reference table or a query parameter.",
};
pub(crate) const REPEATED_JSON_PARSE: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN045",
    message: "JSON payload is parsed more than once in one SELECT",
    remediation: "Parse the payload once in the CTE that reads it, for example `TRY_PARSE_JSON(value) AS payload`, and read every field from that column in a later step.",
};
pub(crate) const DUPLICATE_CTE_NAME: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN046",
    message: "WITH defines the same CTE name more than once",
    remediation: "Give each CTE in one WITH clause its own name; engines reject the duplicate or silently read only one definition.",
};
pub(crate) const NULL_MATCH: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN047",
    message: "CASE WHEN NULL or IN (NULL) never matches",
    remediation: "A simple CASE compares with =, and x IN (NULL) means x = NULL, so neither is ever true. Test the value with IS NULL, for example CASE WHEN x IS NULL THEN ... END.",
};
pub(crate) const LITERAL_ZERO_DIVISION: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN048",
    message: "Expression divides by a literal zero",
    remediation: "Depending on the dialect this raises an error or returns NULL. Remove the division, or divide by NULLIF(divisor, 0) when the divisor can legitimately be zero.",
};
pub(crate) const UNFILTERED_WRITE: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN049",
    message: "UPDATE or DELETE has no limiting predicate and changes every row",
    remediation: "Add a WHERE condition that reads the target rows; a constant such as WHERE TRUE does not limit them. Use TRUNCATE, or an exception for this file, when changing every row is intended.",
};
pub(crate) const INSERT_WITHOUT_COLUMNS: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN050",
    message: "INSERT does not list its target columns",
    remediation: "List the target columns, for example INSERT INTO orders (order_id, status) SELECT ..., so values cannot shift into the wrong column when the table changes.",
};
pub(crate) const NATURAL_JOIN: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN051",
    message: "NATURAL JOIN matches on every column the relations share",
    remediation: "Join with an explicit ON or USING condition; adding a column with a shared name to either relation silently changes a NATURAL JOIN.",
};
pub(crate) const REVERSED_BETWEEN: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN052",
    message: "BETWEEN lower bound is greater than its upper bound",
    remediation: "BETWEEN low AND high is empty when low > high, so it never matches, and NOT BETWEEN matches every non-NULL value. Swap the bounds, or write BETWEEN SYMMETRIC where the dialect supports it.",
};
pub(crate) const MISPLACED_AGGREGATE: LintRuleMetadata = LintRuleMetadata {
    code: "SQLCLEAN053",
    message: "Aggregate or window function is used in WHERE, ON or GROUP BY",
    remediation: "Rows are filtered and grouped before aggregates and windows are computed. Filter an aggregate in HAVING, filter a window result in QUALIFY or an outer query, and group by the underlying columns.",
};
pub(crate) const UNSTABLE_ROW_NUMBER_CODE: &str = "SQLCLEAN018";
pub(crate) const UNPROVEN_RANKING_MESSAGE: &str =
    "Ranking window order is not proven unique within its partition";
pub(crate) const UNPROVEN_RANKING_REMEDIATION: &str = "PARTITION BY and ORDER BY must together cover a unique key of the ranked rows. End the ORDER BY with a unique, non-null tie-breaker column. If the rows have no unique column, build one upstream from the columns that identify a row (for example `HASH(source_partition, source_offset, line_index) AS row_key`) and order by it last.";
/// SQLCLEAN043 guidance before the `max_ranking_order_by` setting snippet.
pub(crate) const RANKING_SORT_GUIDANCE: &str = "Order by the one or two columns that decide which row wins, then a unique row key as the final tie-breaker. If the rows have no unique column, build one upstream from the columns that identify a row (for example `HASH(source_partition, source_offset, line_index) AS row_key`), so SQLCLEAN018 can prove the order is deterministic. If the order genuinely needs more keys, raise `max_ranking_order_by`.";
/// SQLCLEAN044 guidance before the `max_literal_length` setting snippet.
pub(crate) const LONG_LITERAL_GUIDANCE: &str = "Shorten the literal, or move the value into a reference table or a query parameter so it is defined once.";
pub(crate) const RULES_THRESHOLDS_SECTION: &str = "rules.thresholds";
pub(crate) const MAX_RANKING_ORDER_BY_KEY: &str = "max_ranking_order_by";
pub(crate) const MAX_LITERAL_LENGTH_KEY: &str = "max_literal_length";
pub(crate) const DISTINCT_UNUSED_REMEDIATION: &str = "This CTE uses SELECT DISTINCT, so the column still decides which rows are kept. Make the deduplication independent of the column, or restructure the CTE so it outputs only what later steps read.";
pub(crate) const GROUP_KEY_UNUSED_REMEDIATION: &str = "Drop the column from the CTE output but keep it in GROUP BY; the grouping grain stays the same.";
pub(crate) const GROUP_BY_ALL_UNUSED_REMEDIATION: &str = "This CTE uses GROUP BY ALL, so the column is part of the grouping grain. List the grouping columns explicitly and keep this one in GROUP BY, then drop it from the output.";
/// Expression kinds that compute one value per input row without changing the row set.
pub(crate) const ROW_NEUTRAL_EXPRESSIONS: &[&str] = &[
    "literal",
    "boolean",
    "null",
    "identifier",
    "column",
    "paren",
    "dot",
    "subscript",
    "data_type",
    "interval",
    "cast",
    "try_cast",
    "safe_cast",
    "collation",
    "case",
    "and",
    "or",
    "not",
    "neg",
    "add",
    "sub",
    "mul",
    "div",
    "mod",
    "eq",
    "neq",
    "lt",
    "lte",
    "gt",
    "gte",
    "like",
    "i_like",
    "is",
    "is_null",
    "is_true",
    "is_false",
    "in",
    "between",
    "null_safe_eq",
    "bitwise_and",
    "bitwise_or",
    "bitwise_xor",
    "concat",
    "concat_ws",
    "substring",
    "upper",
    "lower",
    "length",
    "trim",
    "replace",
    "left",
    "right",
    "lpad",
    "rpad",
    "split",
    "split_part",
    "regexp_replace",
    "initcap",
    "position",
    "contains",
    "starts_with",
    "ends_with",
    "abs",
    "round",
    "floor",
    "ceil",
    "greatest",
    "least",
    "coalesce",
    "null_if",
    "if_func",
    "if_null",
    "nvl",
    "nvl2",
    "decode",
    "date_add",
    "date_diff",
    "date_trunc",
    "timestamp_trunc",
    "extract",
    "to_date",
    "to_timestamp",
    "to_char",
    "to_number",
    "date",
    "year",
    "month",
    "last_day",
    "struct_extract",
];
/// Call names SQLClean removed as deterministic scalar calls, at the arities the next two lists allow.
pub(crate) const DETERMINISTIC_SCALAR_FUNCTIONS: &[&str] = &[
    "ABS",
    "ARRAY_SIZE",
    "CEIL",
    "CEILING",
    "CHARACTER_LENGTH",
    "CHARINDEX",
    "CHAR_LENGTH",
    "COALESCE",
    "CONCAT",
    "CONCAT_WS",
    "CONTAINS",
    "DATEADD",
    "DATEDIFF",
    "DATE_ADD",
    "DATE_DIFF",
    "DATE_PART",
    "DATE_TRUNC",
    "DIV0",
    "ENDSWITH",
    "ENDS_WITH",
    "EXTRACT",
    "FLOOR",
    "GREATEST",
    "IF",
    "IFF",
    "IFNULL",
    "INITCAP",
    "LAST_DAY",
    "LEAST",
    "LEFT",
    "LEN",
    "LENGTH",
    "LOWER",
    "LPAD",
    "LTRIM",
    "MD5",
    "MONTH",
    "NULLIF",
    "NVL",
    "NVL2",
    "POWER",
    "REGEXP_REPLACE",
    "REPLACE",
    "RIGHT",
    "ROUND",
    "RPAD",
    "RTRIM",
    "SPLIT_PART",
    "STARTSWITH",
    "STARTS_WITH",
    "SUBSTR",
    "SUBSTRING",
    "TEXTLEN",
    "TO_CHAR",
    "TO_DATE",
    "TO_NUMBER",
    "TO_VARCHAR",
    "TRIM",
    "TRUNC",
    "TRY_TO_NUMBER",
    "UPPER",
    "YEAR",
    "ZEROIFNULL",
];
/// Calls SQLClean removed only with one argument; polyglot left other arities as unknown calls.
pub(crate) const SINGLE_ARGUMENT_SCALAR_FUNCTIONS: &[&str] = &[
    "CHARACTER_LENGTH",
    "CHAR_LENGTH",
    "INITCAP",
    "LEN",
    "TEXTLEN",
];
/// Calls SQLClean removed only with more than one argument; polyglot typed the one-argument form.
pub(crate) const MULTI_ARGUMENT_SCALAR_FUNCTIONS: &[&str] = &["ARRAY_SIZE"];
pub(crate) const DEFAULT_MAX_LITERAL_LENGTH: usize = 100;
pub(crate) const DEFAULT_MAX_RANKING_ORDER_BY: usize = 5;
pub(crate) const TIE_SENSITIVE_FUNCTIONS: [&str; 3] = ["ROW_NUMBER", "FIRST_VALUE", "LAST_VALUE"];
pub(crate) const MIN_REMOVABLE_OUTPUTS: usize = 2;
pub(crate) const FIRST_ROW: &str = "1";

/// The keywords that end a select list.
pub(crate) const PROJECTION_ENDS: [TokenType; 13] = [
    TokenType::From,
    TokenType::Where,
    TokenType::Group,
    TokenType::GroupBy,
    TokenType::Having,
    TokenType::Qualify,
    TokenType::Order,
    TokenType::OrderBy,
    TokenType::Limit,
    TokenType::Window,
    TokenType::Union,
    TokenType::Except,
    TokenType::Intersect,
];
