//! Deep nesting ends in the `nesting_depth` gap on a small thread, never a stack overflow.

use crate::parse::tests::helpers::{alternating, flat_chain, gaps_on_small_stack, nested_chains};
use crate::parse::tests::test_types::{DepthTestCase, ParseTimeTestCase};

#[test]
fn given_nesting_or_chains_past_the_limit_when_parsing_on_a_small_stack_then_it_is_the_nesting_depth_gap()
 {
    let levels: usize = 1_500;
    let test_cases = [
        DepthTestCase {
            description: "parentheses",
            sql: format!("SELECT {}1{}", "(".repeat(levels), ")".repeat(levels)),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "subqueries",
            sql: format!(
                "SELECT {}1{}",
                "(SELECT ".repeat(levels),
                ")".repeat(levels)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "IF blocks",
            sql: format!(
                "{}SELECT 1{}",
                "IF 1 = 1 BEGIN ".repeat(levels),
                " END".repeat(levels)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "NOT",
            sql: format!("SELECT 1 WHERE {}1 = 1", "NOT ".repeat(levels)),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "alternating forms",
            sql: alternating(levels),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`+` over numbers with 5,000 links",
            sql: flat_chain("SELECT 1", " + 1", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`+` over numbers with 100,000 links",
            sql: flat_chain("SELECT 1", " + 1", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`+` over strings with 5,000 links",
            sql: flat_chain("SELECT 'a'", " + 'a'", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`+` over strings with 100,000 links",
            sql: flat_chain("SELECT 'a'", " + 'a'", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`*` with 5,000 links",
            sql: flat_chain("SELECT 1", " * 1", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`*` with 100,000 links",
            sql: flat_chain("SELECT 1", " * 1", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "AND with 5,000 links",
            sql: flat_chain("SELECT 1 WHERE 1 = 1", " AND 1 = 1", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "AND with 100,000 links",
            sql: flat_chain("SELECT 1 WHERE 1 = 1", " AND 1 = 1", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "OR with 5,000 links",
            sql: flat_chain("SELECT 1 WHERE 1 = 1", " OR 1 = 1", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "OR with 100,000 links",
            sql: flat_chain("SELECT 1 WHERE 1 = 1", " OR 1 = 1", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "chained BETWEEN with 5,000 links",
            sql: flat_chain(
                "SELECT 1 WHERE 1 BETWEEN 0 AND 2",
                " AND 1 BETWEEN 0 AND 2",
                5_000,
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "chained BETWEEN with 100,000 links",
            sql: flat_chain(
                "SELECT 1 WHERE 1 BETWEEN 0 AND 2",
                " AND 1 BETWEEN 0 AND 2",
                100_000,
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "a `+` chain set to a variable with 5,000 links",
            sql: flat_chain("DECLARE @x int; SET @x = 1", "+1", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "a `+` chain set to a variable with 100,000 links",
            sql: flat_chain("DECLARE @x int; SET @x = 1", "+1", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "UNION ALL with 5,000 links",
            sql: flat_chain("SELECT 1", " UNION ALL SELECT 1", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "UNION ALL with 100,000 links",
            sql: flat_chain("SELECT 1", " UNION ALL SELECT 1", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "INTERSECT with 5,000 links",
            sql: flat_chain("SELECT 1", " INTERSECT SELECT 1", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "INTERSECT with 100,000 links",
            sql: flat_chain("SELECT 1", " INTERSECT SELECT 1", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "AT TIME ZONE suffixes with 5,000 links",
            sql: flat_chain("SELECT SYSDATETIMEOFFSET()", " AT TIME ZONE 'UTC'", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "AT TIME ZONE suffixes with 100,000 links",
            sql: flat_chain("SELECT SYSDATETIMEOFFSET()", " AT TIME ZONE 'UTC'", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "COLLATE suffixes with 5,000 links",
            sql: flat_chain("SELECT 'a'", " COLLATE Latin1_General_CI_AS", 5_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "COLLATE suffixes with 100,000 links",
            sql: flat_chain("SELECT 'a'", " COLLATE Latin1_General_CI_AS", 100_000),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`+` chains of 900 links nested 10 deep in each first operand",
            sql: format!("SELECT {}", nested_chains("1", " + 1", 900, 10)),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "AND chains of 900 links nested 10 deep in each first operand",
            sql: format!(
                "SELECT 1 WHERE {}",
                nested_chains("1 = 1", " AND 1 = 1", 900, 10)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "UNION ALL chains of 900 links nested 10 deep in each first operand",
            sql: nested_chains("SELECT 1", " UNION ALL SELECT 1", 900, 10),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "AT TIME ZONE chains of 900 links nested 10 deep in each first operand",
            sql: format!(
                "SELECT {}",
                nested_chains("SYSDATETIMEOFFSET()", " AT TIME ZONE 'UTC'", 900, 10)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "COLLATE chains of 900 links nested 10 deep in each first operand",
            sql: format!(
                "SELECT {}",
                nested_chains("'a'", " COLLATE Latin1_General_CI_AS", 900, 10)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`+` in an IF block chains of 900 links nested 10 deep in each first operand",
            sql: format!(
                "IF 1 = 1 BEGIN SELECT {} END",
                nested_chains("1", " + 1", 900, 10)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`+` in a common table expression chains of 900 links nested 10 deep in each first operand",
            sql: format!(
                "WITH c AS (SELECT {} AS x) SELECT x FROM c",
                nested_chains("1", " + 1", 900, 10)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "a derived table of 2,000 UNION ALL links",
            sql: format!(
                "SELECT * FROM ({}) s",
                flat_chain("SELECT 1", " UNION ALL SELECT 1", 2_000)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "`+` chains of 900 links nested 10 deep in an OFFSET",
            sql: format!(
                "SELECT 1 ORDER BY 1 OFFSET {} ROWS",
                nested_chains("1", " + 1", 900, 10)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "990 nested IF statements without BEGIN",
            sql: format!("{}SELECT 1", "IF 1 = 1 ".repeat(990)),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "an IF ... ELSE IF chain of 990 branches",
            sql: format!("{}SELECT 1", "IF 1 = 1 SELECT 1 ELSE ".repeat(990)),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
        DepthTestCase {
            description: "a syntax error under the nesting",
            sql: format!(
                "SELECT {}SELECT 1 FROM{}",
                "(".repeat(levels),
                ")".repeat(levels)
            ),
            expected_gaps: Ok(vec!["nesting_depth"]),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            gaps_on_small_stack(test_case.sql),
            test_case.expected_gaps,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_nesting_or_chains_within_the_limit_when_parsing_on_a_small_stack_then_it_is_a_whole_tree()
{
    let test_cases = [
        DepthTestCase {
            description: "190 subqueries, the most SQL Server 2022 accepts",
            sql: format!("SELECT {}1{}", "(SELECT ".repeat(190), ")".repeat(190)),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "450 parentheses, within the 512 limit",
            sql: format!("SELECT {}1{}", "(".repeat(450), ")".repeat(450)),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "`+` over numbers with 450 links",
            sql: flat_chain("SELECT 1", " + 1", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "`+` over strings with 450 links",
            sql: flat_chain("SELECT 'a'", " + 'a'", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "`*` with 450 links",
            sql: flat_chain("SELECT 1", " * 1", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "AND with 450 links",
            sql: flat_chain("SELECT 1 WHERE 1 = 1", " AND 1 = 1", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "OR with 450 links",
            sql: flat_chain("SELECT 1 WHERE 1 = 1", " OR 1 = 1", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "chained BETWEEN with 250 links",
            sql: flat_chain(
                "SELECT 1 WHERE 1 BETWEEN 0 AND 2",
                " AND 1 BETWEEN 0 AND 2",
                250,
            ),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "a `+` chain set to a variable with 450 links",
            sql: flat_chain("DECLARE @x int; SET @x = 1", "+1", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "UNION ALL with 450 links",
            sql: flat_chain("SELECT 1", " UNION ALL SELECT 1", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "INTERSECT with 450 links",
            sql: flat_chain("SELECT 1", " INTERSECT SELECT 1", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "AT TIME ZONE suffixes with 450 links",
            sql: flat_chain("SELECT SYSDATETIMEOFFSET()", " AT TIME ZONE 'UTC'", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "COLLATE suffixes with 450 links",
            sql: flat_chain("SELECT 'a'", " COLLATE Latin1_General_CI_AS", 450),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "450 nested IF statements without BEGIN",
            sql: format!("{}SELECT 1", "IF 1 = 1 ".repeat(450)),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "an IF ... ELSE IF chain of 450 branches",
            sql: format!("{}SELECT 1", "IF 1 = 1 SELECT 1 ELSE ".repeat(450)),
            expected_gaps: Ok(Vec::new()),
        },
        DepthTestCase {
            description: "150 alternating forms",
            sql: alternating(150),
            expected_gaps: Ok(Vec::new()),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            gaps_on_small_stack(test_case.sql),
            test_case.expected_gaps,
            "{}",
            test_case.description
        );
    }
}

#[test]
#[ignore = "timing"]
fn given_tops_whose_parentheses_open_on_a_subquery_when_parsing_then_time_grows_with_nesting_not_doubling()
 {
    let nested_top = |wrap: &dyn Fn(&str) -> String| {
        let mut top: String = "1".to_owned();
        for _ in 0..20 {
            top = wrap(&top);
        }
        format!("SELECT TOP ({top}) * FROM t")
    };
    let test_cases = [
        ParseTimeTestCase {
            description: "20 TOP expressions, each led by a subquery and then an operator",
            sql: nested_top(&|top| format!("(SELECT TOP ({top}) 1 FROM t) + 1")),
            expected_budget_seconds: 2,
            expected_gaps: Ok(Vec::new()),
        },
        ParseTimeTestCase {
            description: "20 TOP subqueries, each filling its parentheses",
            sql: nested_top(&|top| format!("SELECT TOP ({top}) 1 FROM t")),
            expected_budget_seconds: 2,
            expected_gaps: Ok(Vec::new()),
        },
    ];
    for test_case in test_cases {
        let started = std::time::Instant::now();
        let gaps = crate::parse(&test_case.sql)
            .map(|parsed| {
                parsed
                    .gaps
                    .iter()
                    .map(|gap| gap.construct)
                    .collect::<Vec<_>>()
            })
            .map_err(|error| error.message);
        let elapsed = started.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(test_case.expected_budget_seconds),
            "{}: took {elapsed:?}",
            test_case.description
        );
        assert_eq!(gaps, test_case.expected_gaps, "{}", test_case.description);
    }
}
