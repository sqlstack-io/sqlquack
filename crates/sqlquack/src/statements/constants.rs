//! The statements file's format and how refusal reasons map to codes.

use sqlquack_core::refusals::models::RefusalCategory;

/// The file's format name, read back by `syntax-coverage results`.
pub const STATEMENTS_FORMAT: &str = "sqlquack-translation-statements/1";
/// The SQL sizes file's format name, read back by `scripts/check_sql_sizes.py`.
pub const SQL_SIZES_FORMAT: &str = "sqlquack-sql-sizes/1";
/// The code of a refusal no diagnostic code names, such as a missing Python runtime.
pub const UNCODED: &str = "uncoded";
/// The prefix of a project function's refusal reason, before its name.
pub const FUNCTION_REASON_PREFIX: &str = "function ";
/// Every refusal category, matched against the start of a refusal reason.
pub const CATEGORIES: [RefusalCategory; 8] = [
    RefusalCategory::MissingRule,
    RefusalCategory::NotExpressible,
    RefusalCategory::ParseFidelity,
    RefusalCategory::TypeCondition,
    RefusalCategory::UnresolvedName,
    RefusalCategory::ConversionBudget,
    RefusalCategory::SnowflakeRejects,
    RefusalCategory::DivergenceRefused,
];
/// The key prefix of a spelling the construct keys merge, such as `<>` beside `!=`.
pub const SPELLING: &str = "spelling:";
/// The key prefix of a plain function call.
pub const CALL: &str = "call:";
/// The key prefix of a function call over a window.
pub const CALL_OVER: &str = "call_over:";
/// The key prefix of a function call with named arguments.
pub const CALL_NAMED: &str = "call_named:";
/// The key prefix of a function called without parentheses.
pub const CALL_BARE: &str = "call_bare:";
