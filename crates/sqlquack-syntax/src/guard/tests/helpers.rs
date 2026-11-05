use std::sync::Arc;

use serde_json::error::Category;

use crate::guard::_helpers::tokens::enforce_tokens;
use crate::guard::main::enforce_parser_token_stats::enforce_parser_token_stats;
use crate::guard::models::ComplexityGuardOptions;
use crate::tokens::models::Tokenizer;

pub(crate) fn parsed_max_parser_depth(json: &str) -> Result<Option<usize>, Category> {
    serde_json::from_str::<ComplexityGuardOptions>(json)
        .map(|guard| guard.max_parser_depth)
        .map_err(|error| error.classify())
}

pub(crate) fn round_tripped(guard: &ComplexityGuardOptions) -> ComplexityGuardOptions {
    let json = serde_json::to_string(guard).expect("guard options serialize");
    serde_json::from_str(&json).expect("serialized guard options parse")
}

pub(crate) fn guard_errors(
    sql: &str,
    options: &ComplexityGuardOptions,
) -> (Result<(), String>, Result<(), String>) {
    let sql: Arc<str> = Arc::from(sql);
    let tokenizer = Tokenizer::default();
    let public_tokens = tokenizer.tokenize(&sql).expect("the query tokenizes");
    let (parser_tokens, stats) = tokenizer
        .tokenize_for_parser(&sql)
        .expect("the query tokenizes for the parser");
    (
        enforce_parser_token_stats(&parser_tokens, &stats, options).map_err(|e| e.to_string()),
        enforce_tokens(&public_tokens, options).map_err(|e| e.to_string()),
    )
}

const INVALID_LIMITS: &[&str] = &["-1", "true", "1.0", "\"128\"", "18446744073709551616"];
const VALID_LIMITS: &[&str] = &["0", "128", "null"];

pub(crate) fn limit_json_problems(field: &str) -> Vec<String> {
    let accepted_invalid = INVALID_LIMITS
        .iter()
        .filter(|value| {
            serde_json::from_str::<ComplexityGuardOptions>(&format!("{{\"{field}\":{value}}}"))
                .is_ok()
        })
        .map(|value| format!("accepted {value}"));
    let changed_valid = VALID_LIMITS
        .iter()
        .filter(|value| {
            let parsed: ComplexityGuardOptions =
                serde_json::from_str(&format!("{{\"{field}\":{value}}}"))
                    .expect("a valid limit parses");
            serde_json::to_value(parsed).expect("guard options serialize")[field]
                != serde_json::from_str::<serde_json::Value>(value).expect("the value is JSON")
        })
        .map(|value| format!("changed {value}"));
    accepted_invalid.chain(changed_valid).collect()
}

pub(crate) fn guard_json_accepted(json: &str) -> bool {
    serde_json::from_str::<ComplexityGuardOptions>(json).is_ok()
}

/// The default-guarded Snowflake parse of `sql`, or its error message.
pub(crate) fn guarded_parse(sql: &str) -> Result<(), String> {
    crate::parse_statements_with(
        sql,
        crate::Dialect::Snowflake,
        &crate::ParseOptions::default(),
        crate::CallMode::Faithful,
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}
