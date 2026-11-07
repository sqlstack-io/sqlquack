use serde_json::{Map, Value, json};

use crate::lint::main::engine::lint_json;
use crate::lint::tests::test_types::CrossJoinTestCase;

/// The dependency sentinel the cross join cases name `order_policy`.
pub(crate) const POLICY_SENTINEL: &str = "__sqlclean_interpolation_7__";

pub(crate) fn nested_function_sql(depth: usize) -> String {
    format!("SELECT {}1{}", "F(".repeat(depth), ")".repeat(depth))
}

pub(crate) fn nested_from_sql(depth: usize) -> String {
    format!(
        "SELECT * FROM {}orders{}",
        "(".repeat(depth),
        ")".repeat(depth)
    )
}

pub(crate) fn diagnostics(sql: &str) -> Result<Vec<Value>, String> {
    let response = lint_json(
        &json!({
            "version": 1,
            "sql": sql,
            "dialect": "snowflake"
        })
        .to_string(),
    )?;
    diagnostic_values(&response)
}

pub(crate) fn diagnostics_for_rules(sql: &str, rules: &[&str]) -> Result<Vec<Value>, String> {
    diagnostics_for_dialect(sql, "snowflake", rules)
}

pub(crate) fn diagnostics_for_dialect(
    sql: &str,
    dialect: &str,
    rules: &[&str],
) -> Result<Vec<Value>, String> {
    let request = json!({
        "version": 1,
        "sql": sql,
        "dialect": dialect,
        "enabled_rules": rules,
    });
    let response = lint_json(&request.to_string())?;
    diagnostic_values(&response)
}

fn diagnostic_values(response: &str) -> Result<Vec<Value>, String> {
    let payload: Value = serde_json::from_str(response).map_err(|error| error.to_string())?;
    payload["diagnostics"]
        .as_array()
        .cloned()
        .ok_or_else(|| "diagnostics should be an array".to_string())
}

/// The `SQLCLEAN038` messages for one cross join case.
pub(crate) fn cross_join_messages(test_case: &CrossJoinTestCase) -> Result<Vec<String>, String> {
    cross_join_texts(test_case, "message")
}

/// The `SQLCLEAN038` remediations for one cross join case.
pub(crate) fn cross_join_remediations(
    test_case: &CrossJoinTestCase,
) -> Result<Vec<String>, String> {
    cross_join_texts(test_case, "remediation")
}

fn cross_join_texts(test_case: &CrossJoinTestCase, field: &str) -> Result<Vec<String>, String> {
    let keys: Map<String, Value> = test_case
        .one_row_relations
        .iter()
        .map(|name| ((*name).to_owned(), json!([[]])))
        .collect();
    let request = json!({
        "version": 1,
        "sql": test_case.sql,
        "dialect": test_case.dialect,
        "enabled_rules": ["SQLCLEAN038"],
        "relation_keys": keys,
        "relation_names": { POLICY_SENTINEL: "order_policy" },
    });
    let response: Value = serde_json::from_str(&lint_json(&request.to_string())?)
        .map_err(|error| error.to_string())?;
    Ok(response["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|diagnostic| diagnostic[field].as_str().unwrap_or_default().to_owned())
        .collect())
}

/// A host fixture harness like a test runner's: generated CTE prefixes, input stand-ins and an empty relation.
pub(crate) fn fixture_harness() -> Value {
    json!({
        "cte_prefixes": ["__expected__", "__ref__", "__source__", "__macro_actual__", "__macro_expected__"],
        "input_cte_prefixes": ["__ref__", "__source__", "__seed__"],
        "empty_relation": "__empty_fixture",
    })
}
