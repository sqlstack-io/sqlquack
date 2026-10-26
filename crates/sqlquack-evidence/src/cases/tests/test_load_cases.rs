use crate::cases::tests::helpers::{observed_problem, source_sql_of};
use crate::cases::tests::test_types::{LoadTestCase, SourceSqlTestCase};

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
