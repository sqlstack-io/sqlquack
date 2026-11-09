use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::SNOWFLAKE;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::models::Nullability::{NonNull, Nullable, Unknown};
use crate::lineage::tests::helpers::typed_shop_schema;
use crate::lineage::tests::test_types::DialectTypeTestCase;

#[test]
fn given_a_query_when_typing_outputs_then_nulls_and_numbers_follow_the_recorded_engines() {
    let test_cases = [
        DialectTypeTestCase {
            description: "ROLLUP keys are NULL in subtotal rows, whatever the catalog or WHERE says",
            sql: "SELECT id, COUNT(*) AS n FROM orders WHERE id IS NOT NULL GROUP BY ROLLUP (id)",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), Nullable),
                ("n", Some("NUMBER(18,0)"), NonNull),
            ],
        },
        DialectTypeTestCase {
            description: "CUBE and GROUPING SETS keys are NULL in subtotal rows",
            sql: "SELECT id, name FROM customers GROUP BY CUBE (id), GROUPING SETS ((name), ())",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), Nullable),
                ("name", Some("VARCHAR(100)"), Nullable),
            ],
        },
        DialectTypeTestCase {
            description: "a USING key a later RIGHT JOIN NULL-fills is nullable, star included",
            sql: "SELECT id, c.* FROM orders JOIN customers USING (id) RIGHT JOIN customers c ON c.id = 1",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), Nullable),
                ("ID", Some("NUMBER(38,0)"), NonNull),
                ("NAME", Some("VARCHAR(100)"), NonNull),
            ],
        },
        DialectTypeTestCase {
            description: "a star over a NATURAL key a later FULL JOIN NULL-fills is nullable",
            sql: "SELECT * FROM orders NATURAL JOIN customers FULL JOIN events e ON TRUE",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("ID", Some("NUMBER(38,0)"), Nullable),
                ("CUSTOMER_ID", Some("NUMBER(38,0)"), Nullable),
                ("AMOUNT", Some("NUMBER(10,2)"), Nullable),
                ("STATUS", Some("VARCHAR"), Nullable),
                ("NAME", Some("VARCHAR(100)"), Nullable),
                ("PAYLOAD", Some("VARIANT"), Nullable),
            ],
        },
        DialectTypeTestCase {
            description: "Snowflake's ASOF JOIN NULL-fills the right side of unmatched rows",
            sql: "SELECT o.id, c.name FROM orders o ASOF JOIN customers c MATCH_CONDITION (o.id >= c.id)",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("id", Some("NUMBER(38,0)"), NonNull),
                ("name", Some("VARCHAR(100)"), Nullable),
            ],
        },
        DialectTypeTestCase {
            description: "a cast of a VARIANT may be NULL: a JSON null casts to SQL NULL",
            sql: "SELECT payload::VARCHAR AS text, CAST(payload AS VARIANT) AS kept, \
                  PARSE_JSON('null')::VARCHAR AS parsed FROM events",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("text", Some("VARCHAR"), Nullable),
                ("kept", Some("VARIANT"), NonNull),
                ("parsed", Some("VARCHAR"), Nullable),
            ],
        },
        DialectTypeTestCase {
            description: "a fraction is NUMBER(p,s) from its digits and an exponent is left untyped",
            sql: "SELECT 1.5 AS half, 1.50 AS trimmed, 1e3 AS power, \
                  COALESCE(customer_id, 1.5) AS filled FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("half", Some("NUMBER(2,1)"), NonNull),
                ("trimmed", Some("NUMBER(2,1)"), NonNull),
                ("power", None, NonNull),
                ("filled", Some("NUMBER(38,1)"), NonNull),
            ],
        },
        DialectTypeTestCase {
            description: "a union with a fraction never rounds it into NUMBER(38,0)",
            sql: "SELECT customer_id AS value FROM orders UNION ALL SELECT 1.5",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[("value", Some("NUMBER(38,1)"), Unknown)],
        },
        DialectTypeTestCase {
            description: "integer literals, COUNT and ROW_NUMBER take the recorded types",
            sql: "SELECT 7 AS seven, 123 AS wide, COUNT(id) AS n, \
                  ROW_NUMBER() OVER (ORDER BY id) AS rn FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("seven", Some("NUMBER(1,0)"), NonNull),
                ("wide", Some("NUMBER(3,0)"), NonNull),
                ("n", Some("NUMBER(18,0)"), NonNull),
                ("rn", Some("NUMBER(18,0)"), Unknown),
            ],
        },
        DialectTypeTestCase {
            description: "TIMESTAMP follows the profile's TIMESTAMP_TYPE_MAPPING",
            sql: "SELECT CAST(status AS TIMESTAMP) AS at, TO_TIMESTAMP(status) AS parsed FROM orders",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("at", Some("TIMESTAMP_NTZ"), Unknown),
                ("parsed", Some("TIMESTAMP_NTZ"), Unknown),
            ],
        },
        DialectTypeTestCase {
            description: "DATEADD keeps a TIMESTAMP(9)'s type, widens TIME to TIME(9), keeps a DATE's",
            sql: "SELECT DATEADD(day, 1, shipped_at) AS a, DATEADD(d, 1, shipped_on) AS b, \
                  DATEADD(hour, 1, cutoff) AS c, DATEADD(hour, 1, shipped_on) AS d, \
                  DATEADD(minute, 30, pickup_time) AS e, DATEADD(day, 1, packed_at) AS f \
                  FROM shipments",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("a", Some("TIMESTAMP_NTZ(9)"), Unknown),
                ("b", Some("DATE"), Unknown),
                ("c", Some("TIME(9)"), Unknown),
                ("d", None, Unknown),
                ("e", Some("TIME(9)"), Unknown),
                ("f", None, Unknown),
            ],
        },
        DialectTypeTestCase {
            description: "DATEDIFF takes its unit's recorded NUMBER, and TIMEs their own",
            sql: "SELECT DATEDIFF(day, shipped_on, shipped_at) AS a, \
                  DATEDIFF('second', shipped_at, shipped_at) AS b, \
                  DATEDIFF(minute, cutoff, cutoff) AS c FROM shipments",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("a", Some("NUMBER(9,0)"), Unknown),
                ("b", Some("NUMBER(18,0)"), Unknown),
                ("c", Some("NUMBER(9,0)"), Unknown),
            ],
        },
    ];
    let schema = typed_shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: test_case.profile,
            schema: &schema,
            sql: None,
        };
        let lineage = query_lineage(
            test_case.sql,
            test_case.dialect,
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
