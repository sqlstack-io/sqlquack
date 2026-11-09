use crate::helpers::{parquet_run, project_run};
use crate::test_types::{ParquetRunTestCase, ProjectFile, RunTestCase};

/// Orders with a decimal amount and a lower-case quoted column, in a qualified source.
const SOURCES: ProjectFile = ProjectFile {
    path: "sources/shop.yml",
    repository: &[],
    text: "sources:\n  - name: orders\n    database: raw\n    schema: shop\n    columns:\n      - name: id\n        type: INTEGER\n      - name: amount\n        type: NUMBER(10,2)\n      - name: '\"note\"'\n        type: VARCHAR\n  - name: customers\n    columns:\n      - name: id\n        type: INTEGER\n",
};
/// The orders as CSV, its header in lower case and one note empty.
const ORDERS_CSV: ProjectFile = ProjectFile {
    path: "data/orders.csv",
    repository: &[],
    text: "id,amount,note\n2,7.5,late\n1,2.25,\n",
};
const ORDERED: &str = "SELECT id, amount * 2 AS doubled FROM raw.shop.orders ORDER BY id";
const UNORDERED: &str = "SELECT id, \"note\" FROM raw.shop.orders";

#[test]
fn given_fixtures_when_running_a_query_then_it_prints_the_rows_and_checks_them_against_expect() {
    let test_cases = [
        RunTestCase {
            description: "list rows fill columns in order, and NUMBER(10,2) keeps its scale",
            files: &[SOURCES],
            sql: ORDERED,
            flags: &["--fixture", "orders=[[2, 3.5, \"b\"], [1, 1, null]]"],
            expected_exit: Some(0),
            expected_stdout: "ID  DOUBLED\n1   2.00\n2   7.00\n",
            expected_stderr_mention: "sqlquack run: done, 2 rows\n",
        },
        RunTestCase {
            description: "an expected 2 equals the decimal 2.00, as DuckDB casts it to the result's type",
            files: &[SOURCES],
            sql: ORDERED,
            flags: &[
                "--fixture",
                "orders=[[2, 3.5, \"b\"], [1, 1, null]]",
                "--expect",
                "[[1, 2], [2, 7]]",
            ],
            expected_exit: Some(0),
            expected_stdout: "ID  DOUBLED\n1   2.00\n2   7.00\n",
            expected_stderr_mention: "sqlquack run: done, 2 rows as expected\n",
        },
        RunTestCase {
            description: "an ordered query's rows must come in the expected order",
            files: &[SOURCES],
            sql: ORDERED,
            flags: &[
                "--fixture",
                "orders=[[2, 3.5, \"b\"], [1, 1, null]]",
                "--expect",
                "[[2, 7], [1, 2]]",
            ],
            expected_exit: Some(1),
            expected_stdout: "ID  DOUBLED\n1   2.00\n2   7.00\n",
            expected_stderr_mention: "  row 1: expected (2, 7.00)\n  row 1: got      (1, 2.00)\n",
        },
        RunTestCase {
            description: "an unordered query's rows match in any order, NULL equal to NULL",
            files: &[SOURCES],
            sql: UNORDERED,
            flags: &[
                "--fixture",
                "orders=[[2, 3.5, \"b\"], [1, 1, null]]",
                "--expect",
                "[[1, null], [2, \"b\"]]",
            ],
            expected_exit: Some(0),
            expected_stdout: "ID  note\n2   b\n1   NULL\n",
            expected_stderr_mention: "as expected",
        },
        RunTestCase {
            description: "object rows name columns ignoring case where it is unambiguous, a missing one NULL",
            files: &[SOURCES],
            sql: UNORDERED,
            flags: &[
                "--fixture",
                "raw.shop.orders=[{\"ID\": 1}, {\"id\": 2, \"note\": \"b\"}]",
                "--expect",
                "[{\"id\": 1}, {\"id\": 2, \"note\": \"c\"}]",
            ],
            expected_exit: Some(1),
            expected_stdout: "ID  note\n1   NULL\n2   b\n",
            expected_stderr_mention: "  missing    (2, c)\n  unexpected (2, b)\n",
        },
        RunTestCase {
            description: "a double-quoted key names a column exactly, an unquoted one folds to upper case",
            files: &[SOURCES],
            sql: UNORDERED,
            flags: &[
                "--fixture",
                "orders=[{\"\\\"note\\\"\": \"q\", \"id\": 5}]",
                "--expect",
                "[{\"ID\": 5, \"\\\"note\\\"\": \"q\"}]",
            ],
            expected_exit: Some(0),
            expected_stdout: "ID  note\n5   q\n",
            expected_stderr_mention: "done, 1 row as expected\n",
        },
        RunTestCase {
            description: "a CSV fixture's header names columns, and an empty field is NULL",
            files: &[SOURCES, ORDERS_CSV],
            sql: "SELECT id, amount, \"note\" FROM raw.shop.orders ORDER BY id",
            flags: &["--fixture", "orders=./data/orders.csv"],
            expected_exit: Some(0),
            expected_stdout: "ID  AMOUNT  note\n1   2.25    NULL\n2   7.50    late\n",
            expected_stderr_mention: "done, 2 rows",
        },
        RunTestCase {
            description: "a declared table without a fixture is empty",
            files: &[SOURCES],
            sql: "SELECT count(*) AS n FROM customers",
            flags: &[],
            expected_exit: Some(0),
            expected_stdout: "N\n0\n",
            expected_stderr_mention: "done, 1 row\n",
        },
        RunTestCase {
            description: "a fixture neither a list nor a path with a / is an error",
            files: &[SOURCES],
            sql: ORDERED,
            flags: &["--fixture", "orders=orders.csv"],
            expected_exit: Some(2),
            expected_stdout: "",
            expected_stderr_mention: "takes rows as a JSON list, or a path holding a /",
        },
        RunTestCase {
            description: "a fixture for a table no source declares is an error",
            files: &[SOURCES],
            sql: ORDERED,
            flags: &["--fixture", "payments=[]"],
            expected_exit: Some(2),
            expected_stdout: "",
            expected_stderr_mention: "no source of the project declares the table payments",
        },
        RunTestCase {
            description: "a refused query runs nothing and exits 1",
            files: &[SOURCES],
            sql: "SELECT missing FROM raw.shop.orders",
            flags: &[],
            expected_exit: Some(1),
            expected_stdout: "",
            expected_stderr_mention: "sqlquack run: refused, 1 refusal",
        },
        RunTestCase {
            description: "more than one statement is an error",
            files: &[SOURCES],
            sql: "SELECT 1 AS a; SELECT 2 AS b",
            flags: &[],
            expected_exit: Some(2),
            expected_stdout: "",
            expected_stderr_mention: "run takes one query, and the input holds 2 statements",
        },
    ];
    for test_case in test_cases {
        let outcome = project_run(test_case.files, test_case.sql, test_case.flags);
        assert_eq!(
            (outcome.exit_code, outcome.stdout.as_str()),
            (test_case.expected_exit, test_case.expected_stdout),
            "{}: stderr {}",
            test_case.description,
            outcome.stderr
        );
        assert!(
            outcome.stderr.contains(test_case.expected_stderr_mention),
            "{}: stderr {}",
            test_case.description,
            outcome.stderr
        );
    }
}

#[test]
fn given_a_parquet_fixture_when_running_then_its_columns_fill_the_declared_table_by_name() {
    let test_cases = [
        ParquetRunTestCase {
            description: "columns in another order fill the columns of their names, the rest NULL",
            columns: "note VARCHAR, id INTEGER",
            rows: "('x', 3)",
            expected_exit: Some(0),
            expected_stdout: "ID  AMOUNT  note\n3   NULL    x\n",
        },
        ParquetRunTestCase {
            description: "a column the source does not declare is an error",
            columns: "id INTEGER, colour VARCHAR",
            rows: "(3, 'red')",
            expected_exit: Some(2),
            expected_stdout: "",
        },
    ];
    for test_case in test_cases {
        let outcome = parquet_run(
            &[SOURCES],
            (test_case.columns, test_case.rows),
            "SELECT id, amount, \"note\" FROM raw.shop.orders",
        );
        assert_eq!(
            (outcome.exit_code, outcome.stdout.as_str()),
            (test_case.expected_exit, test_case.expected_stdout),
            "{}: stderr {}",
            test_case.description,
            outcome.stderr
        );
    }
}
