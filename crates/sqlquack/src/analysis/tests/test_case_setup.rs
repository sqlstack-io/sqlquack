use crate::analysis::tests::helpers::setup_case_outcome;
use crate::analysis::tests::test_types::SetupCaseTestCase;

const NR: &[&str] = &[
    "CREATE TABLE nr (k NUMBER(38,0), v VARCHAR)",
    "INSERT INTO nr VALUES (1, 'one'), (2, 'two'), (3, NULL)",
];

#[test]
fn given_setup_tables_when_converting_a_case_then_duckdb_replays_them_or_it_refuses() {
    let test_cases = [
        SetupCaseTestCase {
            description: "a table read by the run database and schema placeholders",
            setup: NR,
            query: "SELECT t.v FROM input AS i JOIN {{run_database}}.{{run_schema}}.nr AS t ON t.k = i.a",
            expected_outcome: Ok("one"),
        },
        SetupCaseTestCase {
            description: "a table read by the schema placeholder alone",
            setup: NR,
            query: "SELECT t.v FROM input AS i JOIN {{run_schema}}.nr AS t ON t.k = i.a",
            expected_outcome: Ok("one"),
        },
        SetupCaseTestCase {
            description: "an inserted NULL stays NULL",
            setup: NR,
            query: "SELECT COUNT(v) AS n FROM nr",
            expected_outcome: Ok("2"),
        },
        SetupCaseTestCase {
            description: "a decimal keeps its column's scale",
            setup: &[
                "CREATE TABLE sales (id NUMBER(38,0), amount NUMBER(10,2))",
                "INSERT INTO sales VALUES (1, 2.5), (2, 4.00)",
            ],
            query: "SELECT SUM(amount) AS total FROM sales",
            expected_outcome: Ok("6.50"),
        },
        SetupCaseTestCase {
            description: "a value with more decimals than the column holds would round",
            setup: &[
                "CREATE TABLE sales (amount NUMBER(10,2))",
                "INSERT INTO sales VALUES (2.505)",
            ],
            query: "SELECT amount FROM sales",
            expected_outcome: Err("not a number it holds without rounding"),
        },
        SetupCaseTestCase {
            description: "a value with more whole digits than the column holds",
            setup: &[
                "CREATE TABLE sales (amount NUMBER(3,2))",
                "INSERT INTO sales VALUES (10)",
            ],
            query: "SELECT amount FROM sales",
            expected_outcome: Err("not a number it holds without rounding"),
        },
        SetupCaseTestCase {
            description: "a number into a VARCHAR column",
            setup: &["CREATE TABLE t (v VARCHAR)", "INSERT INTO t VALUES (1)"],
            query: "SELECT v FROM t",
            expected_outcome: Err("other than a string or NULL"),
        },
        SetupCaseTestCase {
            description: "a type other than NUMBER or VARCHAR",
            setup: &["CREATE TABLE t (d DATE)"],
            query: "SELECT d FROM t",
            expected_outcome: Err("which setup does not replay"),
        },
        SetupCaseTestCase {
            description: "a column constraint",
            setup: &["CREATE TABLE t (k NUMBER(38,0) PRIMARY KEY)"],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a clause other than unqualified, typed columns"),
        },
        SetupCaseTestCase {
            description: "a column default",
            setup: &["CREATE TABLE t (k NUMBER(38,0) DEFAULT 1)"],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a clause other than unqualified, typed columns"),
        },
        SetupCaseTestCase {
            description: "a table created twice",
            setup: &[
                "CREATE TABLE t (k NUMBER(38,0))",
                "CREATE TABLE t (v VARCHAR)",
            ],
            query: "SELECT k FROM t",
            expected_outcome: Err("creates T again"),
        },
        SetupCaseTestCase {
            description: "a quoted table name",
            setup: &["CREATE TABLE \"t\" (k NUMBER(38,0))"],
            query: "SELECT k FROM \"t\"",
            expected_outcome: Err("quotes the name t"),
        },
        SetupCaseTestCase {
            description: "an insert naming its columns",
            setup: &[
                "CREATE TABLE t (k NUMBER(38,0))",
                "INSERT INTO t (k) VALUES (1)",
            ],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a clause other than VALUES into an unqualified table"),
        },
        SetupCaseTestCase {
            description: "an insert from a query",
            setup: &["CREATE TABLE t (k NUMBER(38,0))", "INSERT INTO t SELECT 1"],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a clause other than VALUES"),
        },
        SetupCaseTestCase {
            description: "a row with fewer values than columns",
            setup: &[
                "CREATE TABLE t (k NUMBER(38,0), v VARCHAR)",
                "INSERT INTO t VALUES (1)",
            ],
            query: "SELECT k FROM t",
            expected_outcome: Err("has a row of 1 values for 2 columns"),
        },
        SetupCaseTestCase {
            description: "an insert into a table setup did not create",
            setup: &["INSERT INTO t VALUES (1)"],
            query: "SELECT 1 AS k",
            expected_outcome: Err("which setup did not create"),
        },
        SetupCaseTestCase {
            description: "a session variable",
            setup: &["SET n = 10"],
            query: "SELECT $n AS n",
            expected_outcome: Err("is not replayed on DuckDB"),
        },
        SetupCaseTestCase {
            description: "a user-defined type",
            setup: &["CREATE TYPE age AS NUMBER(3,0)"],
            query: "SELECT 1 AS k",
            expected_outcome: Err("is not replayed on DuckDB"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            setup_case_outcome(
                (test_case.setup, test_case.query),
                test_case.expected_outcome
            ),
            test_case
                .expected_outcome
                .map(str::to_owned)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
