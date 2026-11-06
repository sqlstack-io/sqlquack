use crate::cases::tests::helpers::{observed_problem, row_source_count, source_sql_of};
use crate::cases::tests::test_types::{LoadTestCase, RowSourceTestCase, SourceSqlTestCase};

#[test]
fn given_case_file_when_loading_then_format_and_policy_rules_are_enforced() {
    let test_cases = [
        LoadTestCase {
            description: "the base file loads",
            replace: "",
            with: "",
            expected_problem: None,
        },
        LoadTestCase {
            description: "an unknown key is rejected",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\ntolerance = 1",
            expected_problem: Some("unknown field `tolerance`"),
        },
        LoadTestCase {
            description: "a category's required inputs must be covered",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\"]",
            expected_problem: Some("category `string` requires inputs no case covers: empty"),
        },
        LoadTestCase {
            description: "an unknown category is rejected",
            replace: "category = \"string\"",
            with: "category = \"dates\"",
            expected_problem: Some("unknown category `dates`"),
        },
        LoadTestCase {
            description: "an unknown session profile is rejected",
            replace: "session_profile = \"utc-v1\"",
            with: "session_profile = \"local\"",
            expected_problem: Some("unknown session profile `local`"),
        },
        LoadTestCase {
            description: "a query needs a declared row order",
            replace: "order = \"ordered\"",
            with: "",
            expected_problem: Some("a query needs `order = \"ordered\"` or `\"bag\"`"),
        },
        LoadTestCase {
            description: "rows need one value per column",
            replace: "[\"''\", \"'wrap'\"]",
            with: "[\"''\"]",
            expected_problem: Some("every row needs one value per column"),
        },
        LoadTestCase {
            description: "a function needs exactly one return form",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\n\n[[case.function]]\nname = \"wrapped\"\narguments = [{ name = \"t\", type = \"VARCHAR\" }]\nbody = \"t\"",
            expected_problem: Some(
                "function `wrapped`: needs exactly one of `returns` and `returns_table`",
            ),
        },
        LoadTestCase {
            description: "a function body cannot hold its dollar quote",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\n\n[[case.function]]\nname = \"wrapped\"\narguments = [{ name = \"t\", type = \"VARCHAR\" }]\nreturns = \"VARCHAR\"\nbody = \"'$$'\"",
            expected_problem: Some("function `wrapped`: needs a body without `$$`"),
        },
        LoadTestCase {
            description: "case ids are unique per file",
            replace: "id = \"sorted\"",
            with: "id = \"concat\"",
            expected_problem: Some("repeats a case id"),
        },
        LoadTestCase {
            description: "a canonical spelling outside an equivalence file is rejected",
            replace: "expression = \"CONCAT(a, b)\"",
            with: "expression = \"CONCAT(a, b)\"\ncanonical = \"a || b\"",
            expected_problem: Some("`canonical` belongs only in equivalence files"),
        },
        LoadTestCase {
            description: "a file of typing cases loads",
            replace: "session_profile = \"utc-v1\"",
            with: "session_profile = \"utc-v1\"\nevidence = \"typing\"",
            expected_problem: None,
        },
        LoadTestCase {
            description: "a typing case may keep no values",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\nevidence = \"typing\"\nvalues = false",
            expected_problem: None,
        },
        LoadTestCase {
            description: "an unknown evidence kind is rejected",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\nevidence = \"values\"",
            expected_problem: Some("unknown variant `values`"),
        },
        LoadTestCase {
            description: "only a typing case may keep no values",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\nvalues = false",
            expected_problem: Some("only a typing case may record no values"),
        },
        LoadTestCase {
            description: "a typing case cannot expect a divergence",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\nevidence = \"typing\"\nexpected_divergence = \"overflow\"",
            expected_problem: Some("a typing case has no DuckDB result to diverge"),
        },
        LoadTestCase {
            description: "setup of per-run objects, variables and session parameters loads",
            replace: "query = \"SELECT a FROM input ORDER BY a\"",
            with: "query = \"SELECT a FROM input JOIN {{run_database}}.{{run_schema}}.t USING (a) ORDER BY a\"\nsetup = [\"CREATE TABLE t (a VARCHAR)\", \"insert  into t VALUES ('a')\", \"SET (lo, hi) = (1, 2)\", \"ALTER SESSION SET WEEK_START = 1\"]",
            expected_problem: None,
        },
        LoadTestCase {
            description: "a setup statement outside the allowed kinds is rejected",
            replace: "order = \"ordered\"",
            with: "order = \"ordered\"\nsetup = [\"GRANT USAGE ON DATABASE d TO ROLE r\"]",
            expected_problem: Some(
                "setup statement `GRANT USAGE ON DATABASE d TO ROLE r` is not CREATE TABLE, VIEW, TYPE or SEMANTIC VIEW, INSERT INTO, SET, UNSET or ALTER SESSION",
            ),
        },
        LoadTestCase {
            description: "ALTER SESSION of a parameter the profile sets is rejected",
            replace: "order = \"ordered\"",
            with: "order = \"ordered\"\nsetup = [\"alter session set timezone = 'America/New_York'\"]",
            expected_problem: Some(
                "setup statement `alter session set timezone = 'America/New_York'` changes `TIMEZONE`, which the session profile sets or pins",
            ),
        },
        LoadTestCase {
            description: "ALTER SESSION UNSET of a parameter the profile sets is rejected",
            replace: "order = \"ordered\"",
            with: "order = \"ordered\"\nsetup = [\"ALTER SESSION UNSET TIMEZONE\"]",
            expected_problem: Some(
                "setup statement `ALTER SESSION UNSET TIMEZONE` changes `TIMEZONE`, which the session profile sets or pins",
            ),
        },
        LoadTestCase {
            description: "setup beside a case function is rejected",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\nsetup = [\"SET x = 1\"]\n\n[[case.function]]\nname = \"wrapped\"\narguments = [{ name = \"t\", type = \"VARCHAR\" }]\nreturns = \"VARCHAR\"\nbody = \"t\"",
            expected_problem: Some("a case with `setup` cannot also define functions"),
        },
        LoadTestCase {
            description: "a setup entry holding two statements is rejected",
            replace: "order = \"ordered\"",
            with: "order = \"ordered\"\nsetup = [\"SET x = 1; DROP TABLE t\"]",
            expected_problem: Some("setup statement `SET x = 1; DROP TABLE t` must be one statement, without `;`"),
        },
        LoadTestCase {
            description: "an unknown placeholder is rejected",
            replace: "query = \"SELECT a FROM input ORDER BY a\"",
            with: "query = \"SELECT a FROM {{other_database}}.s.t ORDER BY a\"\nsetup = [\"SET x = 1\"]",
            expected_problem: Some("unknown placeholder `{{other_database}}`"),
        },
        LoadTestCase {
            description: "a placeholder in a case without setup is rejected",
            replace: "query = \"SELECT a FROM input ORDER BY a\"",
            with: "query = \"SELECT a FROM {{run_database}}.{{run_schema}}.t ORDER BY a\"",
            expected_problem: Some("placeholder `{{run_database}}` needs a case with `setup`"),
        },
        LoadTestCase {
            description: "an equivalence file needs a canonical spelling for every case",
            replace: "session_profile = \"utc-v1\"",
            with: "session_profile = \"utc-v1\"\nequivalence = \"concat_operator\"",
            expected_problem: Some("an equivalence case needs its `canonical` spelling"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            observed_problem(
                test_case.replace,
                test_case.with,
                test_case.expected_problem
            )
            .as_deref(),
            test_case.expected_problem,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_case_when_expanding_then_inputs_become_typed_rows_before_the_case_sql() {
    let test_cases = [
        SourceSqlTestCase {
            description: "an expression is evaluated per row in input order",
            reference: "string/concat.toml#concat",
            expected_sql: "WITH input AS (\n  \
                SELECT 1 AS ordinal, CAST('gift' AS VARCHAR) AS a, CAST(NULL AS VARCHAR) AS b\n  \
                UNION ALL\n  \
                SELECT 2 AS ordinal, CAST('' AS VARCHAR) AS a, CAST('wrap' AS VARCHAR) AS b\n)\n\
                SELECT CONCAT(a, b) AS value\nFROM input\nORDER BY ordinal",
        },
        SourceSqlTestCase {
            description: "a query follows the input relation unchanged",
            reference: "string/concat.toml#sorted",
            expected_sql: "WITH input AS (\n  \
                SELECT 1 AS ordinal, CAST('b' AS VARCHAR) AS a\n  \
                UNION ALL\n  \
                SELECT 2 AS ordinal, CAST('a' AS VARCHAR) AS a\n)\n\
                SELECT a FROM input ORDER BY a",
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            source_sql_of(test_case.reference).as_deref(),
            Some(test_case.expected_sql),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_multi_row_cases_when_loading_then_only_row_by_row_results_get_single_row_sources() {
    let test_cases = [
        RowSourceTestCase {
            description: "an expression's result rows follow its input rows",
            reference: "string/concat.toml#concat",
            replace: "",
            with: "",
            expected_rows: Some(2),
        },
        RowSourceTestCase {
            description: "an expression holding a subquery couples rows, so it is not split",
            reference: "string/concat.toml#concat",
            replace: "expression = \"CONCAT(a, b)\"",
            with: "expression = \"CONCAT(a, (SELECT MAX(b) FROM input))\"",
            expected_rows: Some(0),
        },
        RowSourceTestCase {
            description: "a query ordered by another column is not split",
            reference: "string/concat.toml#sorted",
            replace: "",
            with: "",
            expected_rows: Some(0),
        },
        RowSourceTestCase {
            description: "a query over input in order is split",
            reference: "string/concat.toml#sorted",
            replace: "SELECT a FROM input ORDER BY a",
            with: "SELECT a FROM input ORDER BY ordinal",
            expected_rows: Some(2),
        },
        RowSourceTestCase {
            description: "an aggregate couples rows, so it is not split",
            reference: "string/concat.toml#sorted",
            replace: "SELECT a FROM input ORDER BY a",
            with: "SELECT COUNT(a) AS n FROM input ORDER BY ordinal",
            expected_rows: Some(0),
        },
        RowSourceTestCase {
            description: "a window couples rows, so it is not split",
            reference: "string/concat.toml#sorted",
            replace: "SELECT a FROM input ORDER BY a",
            with: "SELECT LAG(a) OVER (ORDER BY ordinal) AS p FROM input ORDER BY ordinal",
            expected_rows: Some(0),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            row_source_count(test_case.reference, (test_case.replace, test_case.with)),
            test_case.expected_rows,
            "{}",
            test_case.description
        );
    }
}
