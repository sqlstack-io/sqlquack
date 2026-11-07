use serde_json::{Value, json};

use crate::lint::main::engine::lint_json;
use crate::lint::tests::helpers;
use crate::lint::tests::test_types::{
    DynamicOutputStarLintTestCase, LintTestCase, SqlMacroOutputLintTestCase,
};

#[test]
fn given_sql_macro_output_when_linting_qualification_then_each_column_counts_as_written()
-> Result<(), String> {
    let test_cases = [
        SqlMacroOutputLintTestCase {
            description: "a bare column from an expression macro beside a qualified column fails",
            sql: "SELECT o.order_id, status AS s FROM orders AS o",
            output: "status",
            relations: &[],
            expected_count: 1,
        },
        SqlMacroOutputLintTestCase {
            description: "reads through a passed relation pass beside a bare caller column",
            sql: "SELECT order_id, o.amount - o.discount AS net FROM orders AS o",
            output: "o.amount - o.discount",
            relations: &["o"],
            expected_count: 0,
        },
        SqlMacroOutputLintTestCase {
            description: "reads through a passed relation pass beside a hand-qualified column",
            sql: "SELECT o.order_id, o.amount - o.discount AS net FROM orders AS o",
            output: "o.amount - o.discount",
            relations: &["o"],
            expected_count: 0,
        },
        SqlMacroOutputLintTestCase {
            description: "the caller's own mix beside reads through a passed relation fails",
            sql: "SELECT o.order_id, status, o.amount AS a FROM orders AS o",
            output: "o.amount",
            relations: &["o"],
            expected_count: 1,
        },
        SqlMacroOutputLintTestCase {
            description: "an aliased read counts as qualified beside a bare caller column",
            sql: "SELECT order_id, s.amount AS a FROM orders AS s",
            output: "s.amount",
            relations: &["orders"],
            expected_count: 1,
        },
        SqlMacroOutputLintTestCase {
            description: "a query scope the macro generated whole is judged in the macro file",
            sql: "WITH c AS (SELECT a, w.b FROM w) SELECT * FROM c",
            output: "SELECT a, w.b FROM w",
            relations: &[],
            expected_count: 0,
        },
    ];
    for test_case in test_cases {
        let start: usize = test_case
            .sql
            .find(test_case.output)
            .ok_or("the output should be in the SQL")?;
        let response = lint_json(
            &json!({
                "version": 1,
                "sql": test_case.sql,
                "dialect": "duckdb",
                "enabled_rules": ["SQLCLEAN028"],
                "sql_macro_outputs": [{
                    "start": start,
                    "end": start + test_case.output.len(),
                    "relations": test_case.relations,
                }],
            })
            .to_string(),
        )?;
        let payload: Value = serde_json::from_str(&response).map_err(|error| error.to_string())?;
        let diagnostics = payload["diagnostics"]
            .as_array()
            .ok_or("diagnostics should be an array")?;

        assert_eq!(
            diagnostics.len(),
            test_case.expected_count,
            "{}",
            test_case.description
        );
    }
    Ok(())
}

#[test]
fn given_compiler_proven_dynamic_output_when_linting_then_only_terminal_star_is_exempt()
-> Result<(), String> {
    let test_cases = [DynamicOutputStarLintTestCase {
        description: "unrelated CTE star remains controlled",
        sql: "WITH pivot_input AS (SELECT customer_id, category, amount FROM orders), \
final AS (SELECT * FROM pivot_input PIVOT(MAX(amount) FOR category IN (ANY))), \
unrelated AS (SELECT * FROM pivot_input) SELECT * FROM final",
        expected_count: 1,
        expected_prefix: "unrelated AS (SELECT ",
    }];
    for test_case in test_cases {
        let response = lint_json(
            &json!({
                "version": 1,
                "sql": test_case.sql,
                "dialect": "snowflake",
                "enabled_rules": ["SQLCLEAN021"],
                "allows_dynamic_output_star": true,
            })
            .to_string(),
        )?;
        let payload: Value = serde_json::from_str(&response).map_err(|error| error.to_string())?;
        let diagnostics = payload["diagnostics"]
            .as_array()
            .ok_or("diagnostics should be an array")?;

        assert_eq!(
            diagnostics.len(),
            test_case.expected_count,
            "{}",
            test_case.description
        );
        let start = diagnostics[0]["start"].as_u64().unwrap_or_default() as usize;
        assert!(
            test_case.sql[..start].ends_with(test_case.expected_prefix),
            "{}",
            test_case.description
        );
    }
    Ok(())
}

#[test]
fn given_constant_projection_when_linting_row_selection_then_only_column_dependent_rows_fail()
-> Result<(), String> {
    let test_cases = [
        LintTestCase {
            description: "constant-only projection selects identical rows in any order",
            sql: "SELECT CAST(100 AS DOUBLE) AS measured_value, CAST(NULL AS INTEGER), 0 samples FROM items LIMIT 1",
            expected_codes: &[],
            expected_anchors: &[],
        },
        LintTestCase {
            description: "a column reference inside a cast still makes row selection nondeterministic",
            sql: "SELECT CAST(id AS DOUBLE) AS measured_value FROM items LIMIT 1",
            expected_codes: &["SQLCLEAN004"],
            expected_anchors: &[("SQLCLEAN004", "LIMIT")],
        },
        LintTestCase {
            description: "a function call in a constant-looking projection still needs ordering",
            sql: "SELECT now() AS observed_at FROM items OFFSET 1",
            expected_codes: &["SQLCLEAN004"],
            expected_anchors: &[("SQLCLEAN004", "OFFSET")],
        },
    ];
    for test_case in &test_cases {
        let diagnostics = helpers::diagnostics(test_case.sql)?;
        let found: Vec<(&str, &str)> = diagnostics
            .iter()
            .filter_map(|item| {
                let start = item["start"].as_u64()? as usize;
                let end = item["end"].as_u64()? as usize;
                Some((item["code"].as_str()?, &test_case.sql[start..end]))
            })
            .collect();
        assert_eq!(
            found, test_case.expected_anchors,
            "{}",
            test_case.description
        );
    }
    Ok(())
}
