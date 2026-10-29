use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::FirstValueTestCase;

const TYPED_REFUSAL: &str = "may hold a DATE or timestamp";

#[test]
fn given_a_variant_holding_a_typed_leaf_when_compared_by_text_then_conversion_refuses() {
    let test_cases = [
        FirstValueTestCase {
            description: "a DATE in a VARIANT against its text is refused",
            sql: "SELECT TO_VARIANT('2024-01-01'::DATE) = TO_VARIANT('2024-01-01') AS e",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "objects holding a DATE and its text are refused",
            sql: "SELECT OBJECT_CONSTRUCT('k', '2024-01-01'::DATE) \
                  = OBJECT_CONSTRUCT('k', '2024-01-01') AS e",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "a timestamp in a VARIANT against its text is refused, also with <>",
            sql: "SELECT TO_VARIANT('2024-01-01 10:00:00'::TIMESTAMP_NTZ) \
                  <> TO_VARIANT('2024-01-01 10:00:00.000') AS e",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "a DATE nested in an array, compared with text, is refused",
            sql: "SELECT ARRAY_CONSTRUCT('2024-01-01'::DATE) = PARSE_JSON('[\"2024-01-01\"]') AS e",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "a DATE compared after it passed through a CTE is refused",
            sql: "WITH t AS (SELECT TO_VARIANT('2024-01-01'::DATE) AS v) \
                  SELECT v = PARSE_JSON('\"2024-01-01\"') AS e FROM t",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "PARTITION BY a VARIANT mixing a DATE and its text is refused",
            sql: "SELECT COUNT(*) OVER (PARTITION BY IFF(o = 1, TO_VARIANT('2024-01-01'), \
                  TO_VARIANT('2024-01-01'::DATE))) AS n \
                  FROM (SELECT 1 AS o UNION ALL SELECT 2 AS o) AS t",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "ARRAY_CONTAINS of a DATE in text elements is refused",
            sql: "SELECT ARRAY_CONTAINS(TO_VARIANT('2024-01-01'::DATE), \
                  ARRAY_CONSTRUCT('2024-01-01')) AS e",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "a comparison of values proven free of typed leaves converts beside one",
            sql: "SELECT PARSE_JSON('1') = PARSE_JSON('1') AS e, \
                  TO_VARIANT('2024-01-01'::DATE) AS d",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "ARRAY_CONTAINS over a base table's flattened JSON converts beside a typed leaf",
            sql: "SELECT ARRAY_CONTAINS('a'::VARIANT, ARRAY_CONSTRUCT(f.value:k)) AS c, \
                  OBJECT_CONSTRUCT('d', '2024-01-01'::DATE) AS o \
                  FROM events, LATERAL FLATTEN(input => ARRAY_CONSTRUCT(events.payload)) f",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a CTE column built from parsed JSON compares beside a typed leaf",
            sql: "WITH t AS (SELECT PARSE_JSON('{\"k\":1}') AS v, TO_VARIANT('2024-01-01'::DATE) AS d) \
                  SELECT v = PARSE_JSON('{\"k\":1}') AS e FROM t",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a lateral alias holding a typed leaf, compared by its text, is refused",
            sql: "SELECT TO_VARIANT('2024-01-01'::DATE) AS d, d = PARSE_JSON('\"2024-01-01\"') AS e",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "a star over a CTE keeps its columns proven free of typed leaves",
            sql: "WITH t AS (SELECT PARSE_JSON('1') AS v), u AS (SELECT * FROM t) \
                  SELECT v = PARSE_JSON('1') AS e, TO_VARIANT('2024-01-01'::DATE) AS d FROM u",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a star over a CTE holding a typed leaf keeps it unproven",
            sql: "WITH t AS (SELECT TO_VARIANT('2024-01-01'::DATE) AS v), u AS (SELECT * FROM t) \
                  SELECT v = PARSE_JSON('\"2024-01-01\"') AS e FROM u",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "a UNION column, not proven free, compared beside a typed leaf is refused",
            sql: "SELECT v = PARSE_JSON('1') AS e FROM (SELECT PARSE_JSON('1') AS v UNION ALL \
                  SELECT TO_VARIANT('2024-01-01'::DATE) AS v) AS t",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "VARIANTs without typed leaves still compare by their text",
            sql: "SELECT OBJECT_CONSTRUCT('k', '2024-01-01') = PARSE_JSON('{\"k\":\"2024-01-01\"}') AS e",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a typed leaf without a comparison by text still converts",
            sql: "SELECT TO_VARIANT('2024-01-01'::DATE) AS v",
            expected_outcome: Ok("\"2024-01-01\""),
        },
        FirstValueTestCase {
            description: "ARRAY_AGG DISTINCT of a VARIANT is refused, as DuckDB compares JSON as written",
            sql: "SELECT ARRAY_AGG(DISTINCT v) AS a FROM (SELECT OBJECT_CONSTRUCT('k', '2024-01-01'::DATE) AS v \
                  UNION ALL SELECT PARSE_JSON('{\"k\":\"2024-01-01\"}') AS v) AS t",
            expected_outcome: Err("ARRAY_AGG DISTINCT removing duplicate VARIANT values"),
        },
        FirstValueTestCase {
            description: "ARRAY_SIZE of ARRAY_AGG DISTINCT of a VARIANT is refused",
            sql: "SELECT ARRAY_SIZE(ARRAY_AGG(DISTINCT v)) AS n FROM (SELECT PARSE_JSON('{\"a\":1,\"b\":2}') AS v \
                  UNION ALL SELECT PARSE_JSON('{\"b\":2,\"a\":1}') AS v) AS t",
            expected_outcome: Err("ARRAY_AGG DISTINCT removing duplicate VARIANT values"),
        },
        FirstValueTestCase {
            description: "ANY_VALUE of a VARIANT mixing a typed leaf and its text is refused",
            sql: "SELECT ANY_VALUE(v) AS a FROM (SELECT TO_VARIANT('2024-01-01'::DATE) AS v \
                  UNION ALL SELECT TO_VARIANT('2024-01-01') AS v) AS t",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "MAX_BY of a VARIANT holding a typed leaf is refused",
            sql: "SELECT MAX_BY(v, k) AS a FROM (SELECT 1 AS k, TO_VARIANT('2024-01-01'::DATE) AS v \
                  UNION ALL SELECT 1 AS k, TO_VARIANT('2024-01-01') AS v) AS t",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "an unordered ARRAY_AGG of a VARIANT holding a typed leaf is refused",
            sql: "SELECT ARRAY_AGG(v) AS a FROM (SELECT TO_VARIANT('2024-01-01'::DATE) AS v \
                  UNION ALL SELECT TO_VARIANT('2024-01-01') AS v) AS t",
            expected_outcome: Err(TYPED_REFUSAL),
        },
        FirstValueTestCase {
            description: "ANY_VALUE of a base table's VARIANT still converts",
            sql: "SELECT ANY_VALUE(payload) AS a FROM events WHERE payload:k::VARCHAR = 'a'",
            expected_outcome: Ok("{\"k\":\"a\"}"),
        },
        FirstValueTestCase {
            description: "SELECT DISTINCT over a VARIANT is refused, whose key order DuckDB keeps",
            sql: "SELECT COUNT(*) AS n FROM (SELECT DISTINCT v FROM (SELECT PARSE_JSON('{\"a\":1,\"b\":2}') AS v \
                  UNION ALL SELECT PARSE_JSON('{\"b\":2,\"a\":1}') AS v) AS t) AS u",
            expected_outcome: Err("SELECT DISTINCT removing duplicate VARIANT values"),
        },
        FirstValueTestCase {
            description: "SELECT DISTINCT without a VARIANT still converts",
            sql: "SELECT COUNT(*) AS n FROM (SELECT DISTINCT note FROM customers) AS t",
            expected_outcome: Ok("2"),
        },
        FirstValueTestCase {
            description: "UNION of a VARIANT and NULL removes duplicates and is refused",
            sql: "SELECT COUNT(*) AS n FROM (SELECT PARSE_JSON('1') AS v UNION SELECT NULL AS v) AS t",
            expected_outcome: Err("a set operation removing duplicate VARIANT values"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_first_outcome(test_case.sql, test_case.expected_outcome),
            test_case
                .expected_outcome
                .map(str::to_owned)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
