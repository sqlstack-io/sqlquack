use serde_json::{Value, json};

use crate::lint::main::engine::lint_json;
use crate::lint::tests::helpers::fixture_harness;
use crate::lint::tests::test_types;

#[test]
fn given_empty_fixture_context_when_linting_projected_star_then_only_canonical_inputs_pass()
-> Result<(), String> {
    let test_cases = [
        test_types::EmptyFixtureLintTestCase {
            description: "canonical ref fixture",
            sql: "WITH __ref__orders AS (SELECT * FROM __empty_fixture()) SELECT 1",
            dialect: "snowflake",
            in_fixture_harness: true,
            expected_diagnostic_count: 0,
        },
        test_types::EmptyFixtureLintTestCase {
            description: "canonical source fixture",
            sql: "WITH __source__web__orders AS (SELECT * FROM __empty_fixture()) SELECT 1",
            dialect: "snowflake",
            in_fixture_harness: true,
            expected_diagnostic_count: 0,
        },
        test_types::EmptyFixtureLintTestCase {
            description: "ordinary SQL has no intrinsic exemption",
            sql: "SELECT * FROM __empty_fixture()",
            dialect: "duckdb",
            in_fixture_harness: false,
            expected_diagnostic_count: 1,
        },
        test_types::EmptyFixtureLintTestCase {
            description: "expected fixture remains controlled",
            sql: "WITH __expected__orders AS (SELECT * FROM __empty_fixture()) SELECT 1",
            dialect: "snowflake",
            in_fixture_harness: true,
            expected_diagnostic_count: 1,
        },
        test_types::EmptyFixtureLintTestCase {
            description: "additional projection is not canonical",
            sql: "WITH __ref__orders AS (SELECT *, 1 AS extra FROM __empty_fixture()) SELECT 1",
            dialect: "snowflake",
            in_fixture_harness: true,
            expected_diagnostic_count: 1,
        },
    ];
    for test_case in test_cases {
        let response = lint_json(
            &json!({
                "version": 1,
                "sql": test_case.sql,
                "dialect": test_case.dialect,
                "enabled_rules": ["SQLCLEAN021"],
                "fixture_harness": test_case.in_fixture_harness.then(fixture_harness)
            })
            .to_string(),
        )?;
        let payload: Value = serde_json::from_str(&response).map_err(|error| error.to_string())?;
        assert_eq!(
            payload["diagnostics"].as_array().map_or(0, Vec::len),
            test_case.expected_diagnostic_count,
            "{}",
            test_case.description
        );
    }
    let outside_harness = lint_json(
        &json!({
            "version": 1,
            "sql": "SELECT * FROM __empty_fixture()",
            "dialect": "snowflake",
            "enabled_rules": ["SQLCLEAN021"],
        })
        .to_string(),
    );
    assert!(
        outside_harness.is_err(),
        "Snowflake needs TABLE( ) around a table function outside the fixture harness"
    );
    Ok(())
}
