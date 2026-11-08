use std::thread;

use crate::lint::tests::helpers::diagnostics_for_dialect;
use crate::lint::tests::test_types::LongChainTestCase;

const STACK_BYTES: usize = 2 * 1024 * 1024;
const TERMS: usize = 50_000;

#[test]
fn given_long_operator_chains_under_a_cross_join_when_linting_on_a_small_stack_then_lint_finishes()
{
    let test_cases = [
        LongChainTestCase {
            description: "Snowflake: a projection chain beside a CROSS JOIN",
            dialect: "snowflake",
            template: "SELECT {chain} FROM a CROSS JOIN b",
            term: "1",
            separator: " + ",
            terms: TERMS,
            expected_lints: true,
        },
        LongChainTestCase {
            description: "DuckDB: a projection chain beside a CROSS JOIN",
            dialect: "duckdb",
            template: "SELECT {chain} FROM a CROSS JOIN b",
            term: "1",
            separator: " + ",
            terms: TERMS,
            expected_lints: true,
        },
        LongChainTestCase {
            description: "Snowflake: a chain in a cross-joined CTE's projection",
            dialect: "snowflake",
            template: "WITH c AS (SELECT {chain} AS x FROM t) SELECT c.x FROM a CROSS JOIN c",
            term: "1",
            separator: " + ",
            terms: TERMS,
            expected_lints: true,
        },
        LongChainTestCase {
            description: "Snowflake: a chain in the final projection after a CTE",
            dialect: "snowflake",
            template: "WITH c AS (SELECT t.x FROM t), d AS (SELECT c.x FROM c) SELECT {chain} FROM d CROSS JOIN e",
            term: "1",
            separator: " + ",
            terms: TERMS,
            expected_lints: true,
        },
        LongChainTestCase {
            description: "Snowflake: a set-operation chain is refused by the tree depth guard, not a crash",
            dialect: "snowflake",
            template: "WITH c AS ({chain}), r AS (SELECT c.a FROM c QUALIFY ROW_NUMBER() OVER (PARTITION BY c.a ORDER BY c.a) = 1) SELECT r.a FROM r",
            term: "SELECT t.a FROM t",
            separator: " INTERSECT ",
            terms: TERMS,
            expected_lints: false,
        },
    ];
    for test_case in &test_cases {
        let chain: String = vec![test_case.term; test_case.terms].join(test_case.separator);
        let sql: String = test_case.template.replace("{chain}", &chain);
        let dialect: &'static str = test_case.dialect;
        let linted: bool = thread::Builder::new()
            .stack_size(STACK_BYTES)
            .spawn(move || diagnostics_for_dialect(&sql, dialect, &["SQLCLEAN"]).is_ok())
            .expect("the lint thread should start")
            .join()
            .expect("lint should not panic");
        assert_eq!(
            linted, test_case.expected_lints,
            "{}",
            test_case.description
        );
    }
}
