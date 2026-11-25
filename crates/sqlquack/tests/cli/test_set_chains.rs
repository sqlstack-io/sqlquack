use crate::helpers::{bracket_depth, convert_streams, converted_first_value};
use crate::test_types::{ChainGrowthTestCase, ChainValueTestCase, SetChainTestCase};

#[test]
fn given_a_chain_of_one_set_operator_when_converting_then_its_output_nests_no_deeper_with_more_branches()
 {
    let test_cases = [
        SetChainTestCase {
            description: "four UNION ALL branches",
            operator: " UNION ALL ",
            branch: "SELECT 1 AS a",
            branches: 4,
            expected_depth: 4,
        },
        SetChainTestCase {
            description: "sixty-four UNION ALL branches nest as deep as four",
            operator: " UNION ALL ",
            branch: "SELECT 1 AS a",
            branches: 64,
            expected_depth: 4,
        },
        SetChainTestCase {
            description: "sixty-four UNION branches nest as deep as four",
            operator: " UNION ",
            branch: "SELECT 1 AS a",
            branches: 64,
            expected_depth: 4,
        },
        SetChainTestCase {
            description: "sixty-four EXCEPT branches nest as deep as four",
            operator: " EXCEPT ",
            branch: "SELECT 1 AS a",
            branches: 64,
            expected_depth: 4,
        },
    ];
    for test_case in test_cases {
        let sql = format!(
            "WITH orders AS ({}) SELECT a FROM orders",
            vec![test_case.branch; test_case.branches].join(test_case.operator)
        );

        let (converted, _) = convert_streams(&sql, &["--no-config", "--compact"]);

        assert_eq!(
            bracket_depth(&converted),
            test_case.expected_depth,
            "{}: {converted}",
            test_case.description
        );
    }
}

#[test]
fn given_longer_chains_of_one_set_operator_when_converting_then_each_branch_adds_the_same_output() {
    let test_cases = [
        ChainGrowthTestCase {
            description: "UNION ALL",
            operator: " UNION ALL ",
            branch: "SELECT 1 AS a",
            expected_bytes_per_branch: 73,
        },
        ChainGrowthTestCase {
            description: "UNION",
            operator: " UNION ",
            branch: "SELECT 1 AS a",
            expected_bytes_per_branch: 69,
        },
    ];
    for test_case in test_cases {
        let printed = |branches: usize| {
            let sql = format!(
                "WITH orders AS ({}) SELECT a FROM orders",
                vec![test_case.branch; branches].join(test_case.operator)
            );
            convert_streams(&sql, &["--no-config"]).0.len()
        };

        let (short, middle, long) = (printed(4), printed(16), printed(64));

        // Laid-out text grows with nesting, so a chain nested a level per branch grows
        // quadratically, and every pass over it with it; a flat chain grows linearly.
        assert_eq!(
            ((middle - short) / 12, (long - middle) / 48),
            (
                test_case.expected_bytes_per_branch,
                test_case.expected_bytes_per_branch
            ),
            "{}: {short}, {middle} and {long} bytes for 4, 16 and 64 branches",
            test_case.description
        );
    }
}

#[test]
fn given_set_operator_chains_when_converting_then_duckdb_returns_snowflakes_rows() {
    let test_cases = [
        ChainValueTestCase {
            description: "a UNION ALL chain keeps every row",
            sql: "SELECT CAST(COUNT(*) AS VARCHAR) AS n FROM (SELECT 1 AS a UNION ALL SELECT 1 \
                  UNION ALL SELECT 1 UNION ALL SELECT 2)",
            expected_value: "4",
        },
        ChainValueTestCase {
            description: "a UNION between UNION ALLs still removes the duplicates before it only",
            sql: "SELECT CAST(COUNT(*) AS VARCHAR) AS n FROM (SELECT 1 AS a UNION ALL SELECT 1 \
                  UNION SELECT 1 UNION ALL SELECT 1)",
            expected_value: "2",
        },
        ChainValueTestCase {
            description: "an EXCEPT chain removes left to right",
            sql: "SELECT CAST(COUNT(*) AS VARCHAR) AS n FROM (SELECT 1 AS a UNION ALL SELECT 2 \
                  UNION ALL SELECT 3 EXCEPT SELECT 2 EXCEPT SELECT 3)",
            expected_value: "1",
        },
        ChainValueTestCase {
            description: "text around a multi-byte character keeps its aliases in a chain",
            sql: "SELECT MIN(a || b) AS n FROM (SELECT 'caf\u{e9}' AS a, 'x' as b UNION ALL \
                  SELECT 'na\u{ef}ve' AS a, 'y' as b)",
            expected_value: "caf\u{e9}x",
        },
    ];
    for test_case in test_cases {
        let value = converted_first_value(test_case.sql);

        assert_eq!(value, test_case.expected_value, "{}", test_case.description);
    }
}
