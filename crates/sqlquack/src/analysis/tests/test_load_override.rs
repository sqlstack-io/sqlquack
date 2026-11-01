use crate::analysis::_helpers::override_definition::load_override;
use crate::analysis::tests::test_types::LoadOverrideTestCase;

#[test]
fn given_an_override_file_when_loading_then_its_header_is_read_and_duckdb_binds_its_sql() {
    let test_cases = [
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "parameters with parenthesised types, replacing DIV0",
            text: "FUNCTION (\n  arguments (amount NUMBER(12,2), divisor NUMBER(12,2)),\n  returns NUMBER(12,2),\n  replaces div0,\n  dialect duckdb,\n);\nCASE WHEN divisor = 0 THEN 0 ELSE amount / divisor END\n",
            expected_outcome: Ok(("DIV0", 2)),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "an override without parameters",
            text: "FUNCTION (\n  returns VARCHAR,\n  replaces CURRENT_REGION,\n  dialect duckdb,\n);\n'local'",
            expected_outcome: Ok(("CURRENT_REGION", 0)),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a body in another dialect",
            text: "FUNCTION (\n  arguments (amount NUMBER(12,2)),\n  returns NUMBER(12,2),\n  replaces DIV0,\n  dialect snowflake,\n);\namount",
            expected_outcome: Err("needs `dialect duckdb`"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "no function replaced",
            text: "FUNCTION (\n  arguments (amount NUMBER(12,2)),\n  returns NUMBER(12,2),\n  dialect duckdb,\n);\namount",
            expected_outcome: Err("needs `replaces <FUNCTION>`"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a property SQLQuack does not read",
            text: "FUNCTION (\n  arguments (amount NUMBER(12,2)),\n  volatile true,\n  returns NUMBER(12,2),\n  replaces DIV0,\n  dialect duckdb,\n);\namount",
            expected_outcome: Err("the property `volatile` is not understood"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a body DuckDB does not bind",
            text: "FUNCTION (\n  arguments (amount NUMBER(12,2)),\n  returns NUMBER(12,2),\n  replaces DIV0,\n  dialect duckdb,\n);\nno_such_function(amount)",
            expected_outcome: Err("DuckDB does not bind its SQL"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a body that aggregates, collapsing rows",
            text: "FUNCTION (\n  arguments (x VARCHAR),\n  returns VARCHAR,\n  replaces REVERSE,\n  dialect duckdb,\n);\nmax(x)",
            expected_outcome: Err("as a per-row expression"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a body with a window function, depending on row order",
            text: "FUNCTION (\n  arguments (x VARCHAR),\n  returns VARCHAR,\n  replaces REVERSE,\n  dialect duckdb,\n);\nrow_number() OVER () || x",
            expected_outcome: Err("as a per-row expression"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a body without parameters that aggregates",
            text: "FUNCTION (\n  returns VARCHAR,\n  replaces CURRENT_REGION,\n  dialect duckdb,\n);\nstring_agg('a', ',')",
            expected_outcome: Err("as a per-row expression"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "no body after the header",
            text: "FUNCTION (\n  arguments (amount NUMBER(12,2)),\n  returns NUMBER(12,2),\n  replaces DIV0,\n  dialect duckdb,\n);\n",
            expected_outcome: Err("has no body"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a body that closes the macro and runs further statements",
            text: "FUNCTION (\n  arguments (x VARCHAR),\n  returns VARCHAR,\n  replaces REVERSE,\n  dialect duckdb,\n);\nx); SET TimeZone = 'Asia/Tokyo'; CREATE OR REPLACE MACRO hacked(a) AS (a",
            expected_outcome: Err("not one DuckDB expression"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a body that adds a second select item",
            text: "FUNCTION (\n  arguments (x VARCHAR),\n  returns VARCHAR,\n  replaces REVERSE,\n  dialect duckdb,\n);\nx) AS y, (x",
            expected_outcome: Err("reaches past its parentheses"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a body that adds a WHERE clause",
            text: "FUNCTION (\n  arguments (x VARCHAR),\n  returns VARCHAR,\n  replaces REVERSE,\n  dialect duckdb,\n);\nx) WHERE (1",
            expected_outcome: Err("reaches past its parentheses"),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "an argument name that is not a plain identifier",
            text: "FUNCTION (\n  arguments (\"x\" VARCHAR),\n  returns VARCHAR,\n  replaces REVERSE,\n  dialect duckdb,\n);\nupper(x)",
            expected_outcome: Err("is not a plain identifier"),
        },
        LoadOverrideTestCase {
            name: "we\"ird",
            description: "a name holding a double quote, escaped as an identifier",
            text: "FUNCTION (\n  arguments (x VARCHAR),\n  returns VARCHAR,\n  replaces REVERSE,\n  dialect duckdb,\n);\nupper(x)",
            expected_outcome: Ok(("REVERSE", 1)),
        },
        LoadOverrideTestCase {
            name: "amount_ratio",
            description: "a header never closed",
            text: "FUNCTION (\n  arguments (amount NUMBER(12,2),\n",
            expected_outcome: Err("is not closed"),
        },
    ];
    for test_case in test_cases {
        let outcome = load_override(test_case.name, test_case.text)
            .map(|(replaces, replacement)| (replaces, replacement.arguments.len()))
            .map_err(|reason| {
                test_case
                    .expected_outcome
                    .err()
                    .is_some_and(|mention| reason.contains(mention))
            });
        assert_eq!(
            outcome,
            test_case
                .expected_outcome
                .map(|(replaces, arguments)| (replaces.to_owned(), arguments))
                .map_err(|_| true),
            "{}",
            test_case.description
        );
    }
}
