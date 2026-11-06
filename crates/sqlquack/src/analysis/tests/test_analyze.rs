use sqlquack_core::refusals::models::RefusalCategory;

use crate::analysis::tests::helpers::{converted, refused, summarise};
use crate::analysis::tests::test_types::{AnalysisSummary, AnalyzeTestCase};

#[test]
fn given_sql_and_a_schema_when_analyzing_then_it_converts_or_refuses_with_a_category() {
    let test_cases = [
        AnalyzeTestCase {
            description: "a declared Python function is refused where no Python runtime registers it",
            sql: "SELECT is_gift(note) AS gift FROM customers",
            expected_summary: refused(RefusalCategory::DivergenceRefused),
        },
        AnalyzeTestCase {
            description: "a SQL function named by its reference placeholder is a typed call",
            sql: "SELECT __sqlquack_udf__rounded(amount) + 1 AS r FROM orders",
            expected_summary: converted(),
        },
        AnalyzeTestCase {
            description: "an argument of another type than the parameter is refused",
            sql: "SELECT is_gift(customer_id) AS gift FROM customers",
            expected_summary: refused(RefusalCategory::TypeCondition),
        },
        AnalyzeTestCase {
            description: "a VARIANT crossing into Python is refused until recordings prove it",
            sql: "SELECT payload_size(payload) AS size FROM events",
            expected_summary: refused(RefusalCategory::DivergenceRefused),
        },
        AnalyzeTestCase {
            description: "a table function argument of another type than declared is refused",
            sql: "SELECT l.amount FROM TABLE(large_orders(10)) AS l WHERE l.amount > 0",
            expected_summary: refused(RefusalCategory::TypeCondition),
        },
        AnalyzeTestCase {
            description: "a table function argument of exactly the declared type is a typed relation",
            sql: "SELECT l.amount FROM TABLE(large_orders(CAST(10 AS NUMBER(10,2)))) AS l",
            expected_summary: converted(),
        },
        AnalyzeTestCase {
            description: "hex text cast to BINARY(n) converts, raising for longer bytes as recorded",
            sql: "SELECT CAST('AABB' AS BINARY(2)) AS b",
            expected_summary: converted(),
        },
        AnalyzeTestCase {
            description: "BINARY(0) is refused as a length Snowflake does not declare",
            sql: "SELECT CAST('' AS BINARY(0)) AS b",
            expected_summary: refused(RefusalCategory::TypeCondition),
        },
        AnalyzeTestCase {
            description: "an ORDER BY expression naming an alias that shadows a column uses the alias",
            sql: "SELECT amount * 2 AS amount FROM orders ORDER BY amount + 1",
            expected_summary: converted(),
        },
        AnalyzeTestCase {
            description: "a grouped ORDER BY naming an alias that shadows a column is refused",
            sql: "SELECT customer_id, SUM(amount) AS amount FROM orders GROUP BY customer_id \
                  ORDER BY amount",
            expected_summary: refused(RefusalCategory::UnresolvedName),
        },
        AnalyzeTestCase {
            description: "an ORDER BY window over an alias that shadows a column is refused",
            sql: "SELECT amount + 1 AS amount FROM orders \
                  ORDER BY ROW_NUMBER() OVER (ORDER BY amount)",
            expected_summary: refused(RefusalCategory::UnresolvedName),
        },
        AnalyzeTestCase {
            description: "a HAVING aggregate over a shadowed name reads the FROM column",
            sql: "SELECT customer_id, SUM(amount) AS amount FROM orders GROUP BY customer_id \
                  HAVING MAX(amount) > 1",
            expected_summary: converted(),
        },
        AnalyzeTestCase {
            description: "a QUALIFY window over a shadowed name reads the FROM column",
            sql: "SELECT customer_id, amount + 1 AS amount FROM orders \
                  QUALIFY ROW_NUMBER() OVER (ORDER BY amount) = 1",
            expected_summary: converted(),
        },
        AnalyzeTestCase {
            description: "an ORDER BY expression over an alias that is the column itself converts",
            sql: "SELECT amount AS amount FROM orders ORDER BY amount + 1",
            expected_summary: converted(),
        },
        AnalyzeTestCase {
            description: "NOT IN a set operation with a branch without FROM is refused",
            sql: "SELECT amount FROM orders WHERE amount NOT IN (SELECT 1 UNION ALL SELECT NULL)",
            expected_summary: refused(RefusalCategory::TypeCondition),
        },
        AnalyzeTestCase {
            description: "NOT IN a set operation whose branches read relations converts",
            sql: "SELECT amount FROM orders WHERE amount NOT IN \
                  (SELECT amount FROM orders UNION ALL SELECT amount FROM orders)",
            expected_summary: converted(),
        },
        AnalyzeTestCase {
            description: "a faithful query over typed relations converts",
            sql: "SELECT c.customer_id, SUM(o.amount) AS total FROM customers c \
                  JOIN orders o ON o.customer_id = c.customer_id GROUP BY c.customer_id",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![],
                converted: true,
            },
        },
        AnalyzeTestCase {
            description: "comments after select items and inside expressions are dropped",
            sql: "SELECT amount /* net */ + 1 AS gross, -- trailing\n customer_id -- last\n \
                  FROM orders -- source\n WHERE amount > 0",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![],
                converted: true,
            },
        },
        AnalyzeTestCase {
            description: "IFNULL, a spelling with recorded cases, converts as written",
            sql: "SELECT IFNULL(note, 'none') AS n FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![],
                converted: true,
            },
        },
        AnalyzeTestCase {
            description: "CHAR_LENGTH, not in Snowflake's function reference, is refused",
            sql: "SELECT CHAR_LENGTH(note) AS n FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::MissingRule],
                converted: false,
            },
        },
        AnalyzeTestCase {
            description: "LEN converts as LENGTH through their recorded spelling pair",
            sql: "SELECT LEN(note) AS n FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![],
                converted: true,
            },
        },
        AnalyzeTestCase {
            description: "the RLIKE operator converts through the REGEXP_LIKE rule",
            sql: "SELECT note RLIKE 'a.*' AS r FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![],
                converted: true,
            },
        },
        AnalyzeTestCase {
            description: "a regular expression with a lookahead is refused",
            sql: "SELECT REGEXP_REPLACE(note, 'a(?=b)', 'x') AS r FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::TypeCondition],
                converted: false,
            },
        },
        AnalyzeTestCase {
            description: "a shared regular expression converts",
            sql: "SELECT REGEXP_REPLACE(note, '\\\\s+', '_') AS r FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![],
                converted: true,
            },
        },
        AnalyzeTestCase {
            description: "ILIKE with a column pattern is refused",
            sql: "SELECT note ILIKE note AS r FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::DivergenceRefused],
                converted: false,
            },
        },
        AnalyzeTestCase {
            description: "SUBSTR with a fractional length is refused",
            sql: "SELECT SUBSTR(note, 1, 1.5) AS r FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::TypeCondition],
                converted: false,
            },
        },
        AnalyzeTestCase {
            description: "a relation the schema does not type is an unresolved name",
            sql: "SELECT region FROM stores",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::UnresolvedName],
                converted: false,
            },
        },
        AnalyzeTestCase {
            description: "a construct without a rule is a missing rule",
            sql: "SELECT STRTOK(note, note) AS n FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::MissingRule],
                converted: false,
            },
        },
        AnalyzeTestCase {
            description: "comparing a NUMBER with text needs a divergence this mode refuses",
            sql: "SELECT customer_id = note AS same FROM customers",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::DivergenceRefused],
                converted: false,
            },
        },
        AnalyzeTestCase {
            description: "TOP, which no rule converts, is refused as a missing rule",
            sql: "SELECT TOP 5 order_id FROM orders",
            expected_summary: AnalysisSummary {
                parsed: true,
                statements: 1,
                categories: vec![RefusalCategory::MissingRule],
                converted: false,
            },
        },
        AnalyzeTestCase {
            description: "unparseable SQL is one parse-fidelity refusal",
            sql: "SELECT FROM WHERE",
            expected_summary: AnalysisSummary {
                parsed: false,
                statements: 0,
                categories: vec![RefusalCategory::ParseFidelity],
                converted: false,
            },
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            summarise(test_case.sql),
            test_case.expected_summary,
            "{}",
            test_case.description
        );
    }
}
