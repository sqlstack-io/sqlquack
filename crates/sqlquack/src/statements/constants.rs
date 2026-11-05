//! The statements file's format and how refusal reasons map to codes.

use sqlquack_core::refusals::models::RefusalCategory;

/// The file's format name, read back by `syntax-coverage results`.
pub const STATEMENTS_FORMAT: &str = "sqlquack-translation-statements/1";
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
