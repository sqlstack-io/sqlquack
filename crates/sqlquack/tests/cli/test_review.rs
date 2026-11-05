use crate::helpers::{review_export, review_project_functions, review_repository};
use crate::test_types::{ReviewOutcome, ReviewTestCase};

#[test]
fn given_recorded_cases_when_exporting_for_review_then_each_case_shows_sql_and_exact_results() {
    let test_cases = [
        ReviewTestCase {
            description: "a passed division case shows its verdict, rule and exact recorded values",
            file: "numeric/division.slt",
            expected_lines: &[
                "# case numeric/division.toml#half_way_at_result_scale",
                "# verdict: passed",
                "source snowflake",
                "converted duckdb",
                "query snowflake nosort",
                "# NUMBER(16,6)",
                "-0.007813",
                "0.000000",
            ],
        },
        ReviewTestCase {
            description: "a refusal shows the line and column of the construct it refuses",
            file: "temporal/current_time_batch2.slt",
            expected_lines: &[
                "SELECT SYSTEM$TYPEOF(CURRENT_DATE()) AS value",
                "refused",
                "# outside a rule's type conditions at 4:8: SYSTEM$TYPEOF other than with its storage suffix removed by REGEXP_REPLACE(..., '\\[.*\\]$', ''), since the suffix depends on Snowflake's plan",
            ],
        },
        ReviewTestCase {
            description: "an expected divergence and its warning are visible",
            file: "aggregate/grouped.slt",
            expected_lines: &[
                "# verdict: diverged as expected (numeric_range)",
                "# warning: known divergence numeric_range at fn:SUM",
                "100000000000000000000000000000000000000",
            ],
        },
        ReviewTestCase {
            description: "a source VARIANT compared by text shows the source_variant_json assumption",
            file: "comparison/predicates_batch2.slt",
            expected_lines: &[
                "# case comparison/predicates_batch2.toml#eq_variant_text",
                "# warning: known divergence source_variant_json at node:eq",
            ],
        },
    ];
    let review = review_repository();
    for test_case in test_cases {
        assert_eq!(
            review_export(&review, test_case.file, test_case.expected_lines),
            ReviewOutcome {
                exit_code: Some(0),
                missing_lines: vec![],
            },
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_cases_defining_sql_functions_when_reviewing_then_snowflake_ddl_and_duckdb_macros_show() {
    let test_cases = [ReviewTestCase {
        description: "the source creates the function and the conversion creates its macro first",
        file: "project-functions/sql_functions.slt",
        expected_lines: &[
            "# case project-functions/sql_functions.toml#scalar_nested_call",
            "CREATE FUNCTION order_total(qty NUMBER(10,0), price NUMBER(10,2))",
            "RETURNS TABLE (order_id NUMBER(10,0), amount NUMBER(10,2))",
            "CREATE MACRO \"SKU_LABEL\"(p1) AS (SELECT",
            "CREATE MACRO \"HALF_QUANTITY\"(p1) AS (SELECT",
        ],
    }];
    for test_case in test_cases {
        assert_eq!(
            review_project_functions(test_case.file, test_case.expected_lines),
            ReviewOutcome {
                exit_code: Some(0),
                missing_lines: vec![],
            },
            "{}",
            test_case.description
        );
    }
}
