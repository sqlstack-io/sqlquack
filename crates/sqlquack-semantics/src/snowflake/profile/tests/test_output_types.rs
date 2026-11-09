use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::SNOWFLAKE;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::models::Nullability::{NonNull, Nullable, Unknown};
use crate::lineage::tests::helpers::typed_shop_schema;
use crate::snowflake::profile::tests::test_types::OutputTypeTestCase;
use crate::snowflake::profile::tests::test_types::ProjectsStarTestCase;

#[test]
fn given_a_query_when_typing_outputs_then_types_and_nullability_follow_the_values() {
    let test_cases = [
        OutputTypeTestCase {
            description: "a column the WHERE requires NOT NULL is never NULL in any value reading it",
            sql: "SELECT customer_id, CAST(customer_id AS VARCHAR) AS label, UPPER(status) AS loud, \
                  LOWER('X') AS fixed FROM orders WHERE customer_id IS NOT NULL AND status IS NOT NULL",
            expected_outputs: &[
                ("customer_id", Some("NUMBER(38,0)"), NonNull),
                ("label", Some("VARCHAR"), NonNull),
                ("loud", Some("VARCHAR"), NonNull),
                ("fixed", Some("VARCHAR"), NonNull),
            ],
        },
        OutputTypeTestCase {
            description: "LISTAGG takes the catalog function type; a negation keeps its operand type",
            sql: "SELECT LISTAGG(status) AS statuses, -MAX(id) AS lowest FROM orders",
            expected_outputs: &[
                ("statuses", Some("TEXT"), Unknown),
                ("lowest", Some("NUMBER(38,0)"), Unknown),
            ],
        },
        OutputTypeTestCase {
            description: "a star over an open relation lists the columns known so far",
            sql: "SELECT * FROM events",
            expected_outputs: &[("PAYLOAD", Some("VARIANT"), NonNull)],
        },
        OutputTypeTestCase {
            description: "EXCEPT takes its values from the left but its type from both branches",
            sql: "SELECT id AS key FROM orders EXCEPT SELECT name FROM customers",
            expected_outputs: &[("key", Some("NUMBER(38,5)"), NonNull)],
        },
        OutputTypeTestCase {
            description: "a Snowflake set operation keeps the first branch's kind, as probed",
            sql: "SELECT TRUE AS flag, CAST(amount AS NUMBER(10, 2)) AS cost FROM orders \
                  UNION ALL SELECT 1, CAST(id AS NUMBER(12, 0)) FROM orders",
            expected_outputs: &[
                ("flag", Some("BOOLEAN"), NonNull),
                ("cost", Some("NUMBER(14,2)"), Nullable),
            ],
        },
        OutputTypeTestCase {
            description: "a Snowflake set operation a later BOOLEAN cannot join is untyped; numbers widen",
            sql: "SELECT 1 AS flag, 2 AS key FROM orders UNION ALL SELECT TRUE, amount FROM orders",
            expected_outputs: &[
                ("flag", None, NonNull),
                ("key", Some("NUMBER(10,2)"), Nullable),
            ],
        },
        OutputTypeTestCase {
            description: "an aggregate without GROUP BY may be NULL over a filtered column",
            sql: "SELECT MAX(customer_id) AS top FROM orders WHERE customer_id IS NOT NULL",
            expected_outputs: &[("top", Some("NUMBER(38,0)"), Unknown)],
        },
        OutputTypeTestCase {
            description: "stored columns keep the catalog's spelling and nullability",
            sql: "SELECT id, amount, status FROM orders",
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), NonNull),
                ("amount", Some("NUMBER(10,2)"), Nullable),
                ("status", Some("VARCHAR"), Unknown),
            ],
        },
        OutputTypeTestCase {
            description: "casts spell their target without spaces and keep the value's nullability",
            sql: "SELECT CAST(id AS NUMBER(18, 0)) AS a, amount::DECIMAL(9, 4) AS b, \
                  TRY_CAST(id AS INT) AS c, status::VARCHAR(10) AS d FROM orders",
            expected_outputs: &[
                ("a", Some("NUMBER(18,0)"), NonNull),
                ("b", Some("DECIMAL(9,4)"), Nullable),
                ("c", Some("INT"), Unknown),
                ("d", Some("VARCHAR(10)"), Unknown),
            ],
        },
        OutputTypeTestCase {
            description: "literals, NULL, comparisons and IS NULL",
            sql: "SELECT 1 AS n, 1.5 AS d, 'x' AS s, TRUE AS b, NULL AS z, \
                  amount > 1 AS gt, amount IS NULL AS missing FROM orders",
            expected_outputs: &[
                ("n", Some("NUMBER(1,0)"), NonNull),
                ("d", Some("NUMBER(2,1)"), NonNull),
                ("s", Some("VARCHAR"), NonNull),
                ("b", Some("BOOLEAN"), NonNull),
                ("z", None, Nullable),
                ("gt", Some("BOOLEAN"), Unknown),
                ("missing", Some("BOOLEAN"), NonNull),
            ],
        },
        OutputTypeTestCase {
            description: "COUNT is never NULL, COUNT_IF is NULL over no rows; MIN and MAX keep their argument's type",
            sql: "SELECT COUNT(*) AS n, COUNT_IF(amount > 1) AS big, MAX(amount) AS top, \
                  MIN(status) AS first_status FROM orders",
            expected_outputs: &[
                ("n", Some("NUMBER(18,0)"), NonNull),
                ("big", None, Unknown),
                ("top", Some("NUMBER(10,2)"), Unknown),
                ("first_status", Some("VARCHAR"), Unknown),
            ],
        },
        OutputTypeTestCase {
            description: "COALESCE is never NULL with one argument that never is",
            sql: "SELECT COALESCE(amount, CAST(0 AS NUMBER(10, 2))) AS a, COALESCE(amount, NULL) AS b, \
                  COALESCE(status, 'none') AS c FROM orders",
            expected_outputs: &[
                ("a", Some("NUMBER(10,2)"), NonNull),
                ("b", Some("NUMBER(10,2)"), Nullable),
                ("c", Some("VARCHAR"), NonNull),
            ],
        },
        OutputTypeTestCase {
            description: "CASE and IFF unify their results' types; the catalog's rules decide nullability",
            sql: "SELECT CASE WHEN amount > 1 THEN status ELSE 'small' END AS c, \
                  IFF(amount > 1, id, CAST(0 AS NUMBER(38, 0))) AS i, IFF(amount > 1, id, amount) AS j, \
                  UPPER(status) AS u, TO_JSON(status) AS t FROM orders",
            expected_outputs: &[
                ("c", Some("VARCHAR"), Unknown),
                ("i", Some("NUMBER(38,0)"), NonNull),
                ("j", Some("NUMBER(38,2)"), Nullable),
                ("u", Some("VARCHAR"), Unknown),
                ("t", Some("TEXT"), Unknown),
            ],
        },
        OutputTypeTestCase {
            description: "the NULL-filled side of an outer join may be NULL",
            sql: "SELECT o.id, c.name FROM orders o LEFT JOIN customers c ON o.customer_id = c.id",
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), NonNull),
                ("name", Some("VARCHAR(100)"), Nullable),
            ],
        },
        OutputTypeTestCase {
            description: "a right join NULL-fills the items before it",
            sql: "SELECT o.id, c.name FROM orders o RIGHT JOIN customers c ON o.customer_id = c.id",
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), Nullable),
                ("name", Some("VARCHAR(100)"), NonNull),
            ],
        },
        OutputTypeTestCase {
            description: "a WHERE that requires a projected column IS NOT NULL proves it",
            sql: "SELECT o.amount, o.status FROM orders o \
                  WHERE o.amount IS NOT NULL AND NOT (o.status IS NULL)",
            expected_outputs: &[
                ("amount", Some("NUMBER(10,2)"), NonNull),
                ("status", Some("VARCHAR"), NonNull),
            ],
        },
        OutputTypeTestCase {
            description: "types and nullability pass through CTEs and stars",
            sql: "WITH a AS (SELECT id, COUNT(*) AS n FROM orders GROUP BY id) \
                  SELECT * FROM a",
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), NonNull),
                ("n", Some("NUMBER(18,0)"), NonNull),
            ],
        },
        OutputTypeTestCase {
            description: "a union branch that may be NULL makes the column nullable",
            sql: "SELECT id, amount FROM orders UNION ALL SELECT id, NULL FROM customers",
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), NonNull),
                ("amount", Some("NUMBER(10,2)"), Nullable),
            ],
        },
        OutputTypeTestCase {
            description: "a type assertion types a value whose type is otherwise unknown",
            sql: "SELECT __polyglot_typed(shop_udf(status), 'VARCHAR(5)') AS a, \
                  __polyglot_typed(id, 'VARCHAR') AS b, __polyglot_typed_like(shop_udf(status), id) AS c \
                  FROM orders",
            expected_outputs: &[
                ("a", Some("VARCHAR(5)"), Unknown),
                ("b", Some("NUMBER(38,0)"), NonNull),
                ("c", Some("NUMBER(38,0)"), Unknown),
            ],
        },
        OutputTypeTestCase {
            description: "branches agree on a type when their spellings denote one Snowflake type",
            sql: "SELECT status, CAST(id AS INT) AS n, NULL::ARRAY AS a FROM orders \
                  UNION ALL SELECT NULL::VARCHAR(16777216), id, NULL::OBJECT FROM customers",
            expected_outputs: &[
                ("status", Some("VARCHAR"), Nullable),
                ("n", Some("INT"), NonNull),
                ("a", None, Nullable),
            ],
        },
        OutputTypeTestCase {
            description: "a column one branch of a BY NAME union lacks is NULL there",
            sql: "SELECT id, status FROM orders UNION ALL BY NAME SELECT id, name FROM customers",
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), NonNull),
                ("status", Some("VARCHAR"), Nullable),
                ("name", Some("VARCHAR(100)"), Nullable),
            ],
        },
        OutputTypeTestCase {
            description: "functions typed by one argument, and JSON parsing",
            sql: "SELECT LEAD(amount) OVER (ORDER BY id) AS next_amount, NULLIF(status, 'x') AS s, \
                  DATE_TRUNC('day', CAST(id AS TIMESTAMP_NTZ)) AS d, TRY_PARSE_JSON(status) AS j \
                  FROM orders",
            expected_outputs: &[
                ("next_amount", Some("NUMBER(10,2)"), Unknown),
                ("s", Some("VARCHAR"), Unknown),
                ("d", Some("TIMESTAMPNTZ"), Unknown),
                ("j", Some("VARIANT"), Unknown),
            ],
        },
        OutputTypeTestCase {
            description: "WITHIN GROUP keeps its aggregate's type; Snowflake arithmetic follows the recorded rules",
            sql: "SELECT LISTAGG(status, ',') WITHIN GROUP (ORDER BY id) AS l, \
                  CAST(amount AS DOUBLE) / id AS ratio, amount / id AS exact, \
                  CAST(id AS INT) + 1 AS next_id, amount + amount AS doubled FROM orders GROUP BY 3",
            expected_outputs: &[
                ("l", Some("TEXT"), Unknown),
                ("ratio", Some("FLOAT"), Unknown),
                ("exact", Some("NUMBER(16,8)"), Unknown),
                ("next_id", Some("NUMBER(38,0)"), Unknown),
                ("doubled", Some("NUMBER(11,2)"), Unknown),
            ],
        },
        OutputTypeTestCase {
            description: "an exponent literal is left untyped: Snowflake types it NUMBER by its value",
            sql: "SELECT 1e5 AS big, amount * 1e-12 AS tiny FROM orders",
            expected_outputs: &[("big", None, NonNull), ("tiny", None, Unknown)],
        },
        OutputTypeTestCase {
            description: "a relation whose columns are not all known still types the ones it knows",
            sql: "SELECT payload, kind FROM events",
            expected_outputs: &[
                ("payload", Some("VARIANT"), NonNull),
                ("kind", None, Unknown),
            ],
        },
    ];

    let schema = typed_shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: &SNOWFLAKE,
            schema: &schema,
            sql: None,
        };
        let lineage = query_lineage(
            test_case.sql,
            Dialect::Snowflake,
            &ParseOptions::default(),
            context,
        )
        .unwrap_or_else(|error| panic!("{}: {error:?}", test_case.description));
        let outputs: Vec<(&str, Option<&str>, _)> = lineage
            .outputs
            .iter()
            .map(|output| {
                (
                    output.name.as_str(),
                    output.data_type.as_deref(),
                    output.nullability,
                )
            })
            .collect();
        assert_eq!(
            outputs,
            test_case.expected_outputs.to_vec(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_query_when_tracing_lineage_then_a_star_at_its_outermost_level_is_reported() {
    let test_cases = [
        ProjectsStarTestCase {
            description: "a final star over a CTE",
            sql: "WITH a AS (SELECT id FROM orders) SELECT * FROM a",
            expected_projects_star: true,
        },
        ProjectsStarTestCase {
            description: "a qualified star",
            sql: "SELECT o.*, 1 AS one FROM orders o",
            expected_projects_star: true,
        },
        ProjectsStarTestCase {
            description: "a star only inside a CTE",
            sql: "WITH a AS (SELECT * FROM orders) SELECT id FROM a",
            expected_projects_star: false,
        },
        ProjectsStarTestCase {
            description: "a union whose first branch has a star",
            sql: "SELECT * FROM orders UNION ALL SELECT * FROM orders",
            expected_projects_star: true,
        },
        ProjectsStarTestCase {
            description: "a star inside an aggregate",
            sql: "SELECT COUNT(*) AS n FROM orders",
            expected_projects_star: false,
        },
    ];
    let schema = typed_shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: &SNOWFLAKE,
            schema: &schema,
            sql: None,
        };
        let lineage = query_lineage(
            test_case.sql,
            Dialect::Snowflake,
            &ParseOptions::default(),
            context,
        )
        .unwrap_or_else(|error| panic!("{}: {error:?}", test_case.description));
        assert_eq!(
            lineage.projects_star, test_case.expected_projects_star,
            "{}",
            test_case.description
        );
    }
}
