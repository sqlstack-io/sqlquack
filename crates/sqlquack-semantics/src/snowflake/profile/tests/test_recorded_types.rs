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
            description: "DATEADD widens a timestamp to precision 9 and TIME to TIME(9), keeps a DATE's",
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
                ("f", Some("TIMESTAMP_NTZ(9)"), Unknown),
            ],
        },
        DialectTypeTestCase {
            description: "a cast to TIME keeps the precision it states",
            sql: "SELECT CAST(cutoff AS TIME(3)) AS a, cutoff::TIME AS b, '10:00'::TIME(0) AS c \
                  FROM shipments",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("a", Some("TIME(3)"), NonNull),
                ("b", Some("TIME"), NonNull),
                ("c", Some("TIME(0)"), NonNull),
            ],
        },
        DialectTypeTestCase {
            description: "TO_VARIANT of a non-null value casts to non-null text only, other VARIANTs may be NULL",
            sql: "SELECT TO_VARIANT(c.name)::VARCHAR AS a, TO_VARIANT(e.payload)::VARCHAR AS b, \
                  e.payload::VARCHAR AS c, TO_VARIANT(c.name)::STRING AS d, \
                  TO_VARIANT(c.id)::INT AS e FROM customers c, events e",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("a", Some("VARCHAR"), NonNull),
                ("b", Some("VARCHAR"), Nullable),
                ("c", Some("VARCHAR"), Nullable),
                ("d", Some("VARCHAR"), NonNull),
                ("e", Some("INT"), Nullable),
            ],
        },
        DialectTypeTestCase {
            description: "TO_VARIANT of each non-JSON family casts to text as null as its value",
            sql: "SELECT TO_VARIANT(c.name)::VARCHAR AS text, TO_VARIANT('null')::VARCHAR AS null_word, \
                  TO_VARIANT(c.id)::VARCHAR AS number, TO_VARIANT(1.5::FLOAT)::VARCHAR AS float, \
                  TO_VARIANT(TRUE)::VARCHAR AS boolean, TO_VARIANT(s.shipped_on)::VARCHAR AS date, \
                  TO_VARIANT(s.cutoff)::VARCHAR AS time, TO_VARIANT(s.shipped_at)::VARCHAR AS ts, \
                  TO_VARIANT(o.amount)::VARCHAR AS nullable_number \
                  FROM customers c, shipments s, orders o",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("text", Some("VARCHAR"), NonNull),
                ("null_word", Some("VARCHAR"), NonNull),
                ("number", Some("VARCHAR"), NonNull),
                ("float", Some("VARCHAR"), NonNull),
                ("boolean", Some("VARCHAR"), NonNull),
                ("date", Some("VARCHAR"), NonNull),
                ("time", Some("VARCHAR"), NonNull),
                ("ts", Some("VARCHAR"), NonNull),
                ("nullable_number", Some("VARCHAR"), Nullable),
            ],
        },
        DialectTypeTestCase {
            description: "TO_VARIANT of a VARIANT, which may be a JSON null, or a path into one, may cast to NULL; \
                          a whole OBJECT or ARRAY never is a JSON null",
            sql: "SELECT TO_VARIANT(e.payload)::VARCHAR AS variant, \
                  TO_VARIANT(PARSE_JSON('null'))::VARCHAR AS parsed, \
                  TO_VARIANT(c.name):a::VARCHAR AS path, \
                  TO_VARIANT('{}'::OBJECT)::VARCHAR AS object, TO_VARIANT('[]'::ARRAY)::VARCHAR AS array \
                  FROM customers c, events e",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("variant", Some("VARCHAR"), Nullable),
                ("parsed", Some("VARCHAR"), Nullable),
                ("path", Some("VARCHAR"), Unknown),
                ("object", Some("VARCHAR"), NonNull),
                ("array", Some("VARCHAR"), NonNull),
            ],
        },
        DialectTypeTestCase {
            description: "a TO_VARIANT column of a CTE, filtered IS NOT NULL, casts to non-null text",
            sql: "WITH d AS (SELECT DISTINCT TO_VARIANT(o.status) AS tag_value FROM orders o), \
                  p AS (SELECT DISTINCT PARSE_JSON(o.status) AS tag_value FROM orders o) \
                  SELECT d.tag_value::VARCHAR AS tag_text, p.tag_value::VARCHAR AS parsed_text \
                  FROM d, p WHERE d.tag_value IS NOT NULL AND p.tag_value IS NOT NULL",
            dialect: Dialect::Snowflake,
            profile: &SNOWFLAKE,
            expected_outputs: &[
                ("tag_text", Some("VARCHAR"), NonNull),
                ("parsed_text", Some("VARCHAR"), Nullable),
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
