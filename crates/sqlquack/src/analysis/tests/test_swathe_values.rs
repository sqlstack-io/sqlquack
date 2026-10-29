use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::FirstValueTestCase;

#[test]
fn given_swathe_shapes_when_converting_then_proven_values_return_and_the_rest_raise() {
    let test_cases = [
        FirstValueTestCase {
            description: "SUM of FLOAT multiples of a power of two adds exactly in any order",
            sql: "SELECT SUM(x) AS s FROM (SELECT 1.5::FLOAT AS x UNION ALL SELECT 2.25::FLOAT \
                  UNION ALL SELECT -0.5::FLOAT) AS t",
            expected_outcome: Ok("3.25"),
        },
        FirstValueTestCase {
            description: "SUM of two FLOAT values is one commutative addition",
            sql: "SELECT SUM(x) AS s FROM (SELECT 0.1::FLOAT AS x UNION ALL SELECT 0.2::FLOAT) AS t",
            expected_outcome: Ok("0.30000000000000004"),
        },
        FirstValueTestCase {
            description: "SUM of three FLOAT fractions depends on order, so it raises",
            sql: "SELECT SUM(x) AS s FROM (SELECT 0.1::FLOAT AS x UNION ALL SELECT 0.2::FLOAT \
                  UNION ALL SELECT 0.3::FLOAT) AS t",
            expected_outcome: Err("float_sum_order"),
        },
        FirstValueTestCase {
            description: "TRY_HEX_DECODE_STRING decodes pairs of hexadecimal digits",
            sql: "SELECT TRY_HEX_DECODE_STRING('4a4B') AS s",
            expected_outcome: Ok("JK"),
        },
        FirstValueTestCase {
            description: "REGEXP_LIKE with a pattern column in the recorded syntax runs",
            sql: "SELECT REGEXP_LIKE(s, p) AS m FROM (SELECT 'yielding' AS s, '[a-z]+ing' AS p) AS t",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "REGEXP_LIKE with a NULL pattern from a column is NULL",
            sql: "SELECT CAST(REGEXP_LIKE(s, p) AS VARCHAR) IS NULL AS m FROM \
                  (SELECT 'x' AS s, CAST(NULL AS VARCHAR) AS p) AS t",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "REGEXP_LIKE with a pattern column outside the recorded syntax raises",
            sql: "SELECT REGEXP_LIKE(s, p) AS m FROM (SELECT 'aa' AS s, 'a{2}' AS p) AS t",
            expected_outcome: Err("regexp_pattern_syntax"),
        },
        FirstValueTestCase {
            description: "MAX_BY skips NULL keys and keeps the top row's value",
            sql: "SELECT MAX_BY(x, k) AS r FROM (SELECT 'low' AS x, 1 AS k UNION ALL \
                  SELECT 'top' AS x, 3 AS k UNION ALL SELECT 'none' AS x, NULL AS k) AS t",
            expected_outcome: Ok("top"),
        },
        FirstValueTestCase {
            description: "MIN_BY over keys tied with different values raises",
            sql: "SELECT MIN_BY(x, k) AS r FROM (SELECT 'one' AS x, 1 AS k UNION ALL \
                  SELECT 'two' AS x, 1 AS k) AS t",
            expected_outcome: Err("extreme_key_ties"),
        },
        FirstValueTestCase {
            description: "ARRAY_AGG without WITHIN GROUP of one distinct value converts",
            sql: "SELECT ARRAY_AGG(x) AS a FROM (SELECT 'k' AS x UNION ALL SELECT 'k' AS x \
                  UNION ALL SELECT NULL AS x) AS t",
            expected_outcome: Ok("[\"k\",\"k\"]"),
        },
        FirstValueTestCase {
            description: "ARRAY_AGG without WITHIN GROUP of several values raises",
            sql: "SELECT ARRAY_AGG(x) AS a FROM (SELECT 'a' AS x UNION ALL SELECT 'b' AS x) AS t",
            expected_outcome: Err("array_agg_order"),
        },
        FirstValueTestCase {
            description: "ARRAY_SIZE of ARRAY_AGG DISTINCT counts the distinct non-NULL values",
            sql: "SELECT ARRAY_SIZE(ARRAY_AGG(DISTINCT x)) AS n FROM (SELECT 'a' AS x \
                  UNION ALL SELECT 'b' AS x UNION ALL SELECT 'a' AS x) AS t",
            expected_outcome: Ok("2"),
        },
        FirstValueTestCase {
            description: "ARRAY_SORT of ARRAY_AGG DISTINCT orders the values ascending",
            sql: "SELECT ARRAY_SORT(ARRAY_AGG(DISTINCT x)) AS a FROM (SELECT 'b' AS x \
                  UNION ALL SELECT 'a' AS x UNION ALL SELECT 'b' AS x) AS t",
            expected_outcome: Ok("[\"a\",\"b\"]"),
        },
        FirstValueTestCase {
            description: "VARIANT = VARIANT ignores key order",
            sql: "SELECT PARSE_JSON('{\"a\":1,\"b\":2}') = PARSE_JSON('{\"b\":2,\"a\":1}') AS e",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "VARIANT = VARIANT tells a number from its text",
            sql: "SELECT PARSE_JSON('1') = PARSE_JSON('\"1\"') AS e",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "PARTITION BY a VARIANT groups values Snowflake writes alike",
            sql: "SELECT MAX(n) AS m FROM (SELECT ROW_NUMBER() OVER (PARTITION BY v ORDER BY o) \
                  AS n FROM (SELECT 1 AS o, PARSE_JSON('{\"a\":1,\"b\":2}') AS v UNION ALL \
                  SELECT 2 AS o, PARSE_JSON('{\"b\":2,\"a\":1}') AS v) AS t) AS u",
            expected_outcome: Ok("2"),
        },
        FirstValueTestCase {
            description: "text compared with a NUMBER reads spaces, signs and zeros as Snowflake does",
            sql: "SELECT ' +007.0' = 7 AND '1.50' = 1.5 AS e",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "text is compared unrounded, so a fraction differs from an integer",
            sql: "SELECT '7.4' = 7 AS e",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "text with more fraction digits than the widest scale raises",
            sql: "SELECT '7.4' = CAST(7 AS NUMBER(38,0)) AS e",
            expected_outcome: Err("text_number_scale"),
        },
        FirstValueTestCase {
            description: "a scaled NUMBER past 20 whole digits compared with text raises text_number_scale",
            sql: "SELECT '7' = CAST(120000000000000000000 AS NUMBER(30,9)) AS e",
            expected_outcome: Err("text_number_scale"),
        },
        FirstValueTestCase {
            description: "a scaled NUMBER of a wide type within 20 whole digits compares with text",
            sql: "SELECT '7.5' = CAST(7.5 AS NUMBER(30,9)) AS e",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a 38-digit NUMBER compared with text fails as error 100046",
            sql: "SELECT '7' = CAST(99999999999999999999999999999999999999 AS NUMBER(38,0)) AS e",
            expected_outcome: Err("100046"),
        },
        FirstValueTestCase {
            description: "text Snowflake cannot read as a number fails as error 100038",
            sql: "SELECT 'x' = 7 AS e",
            expected_outcome: Err("100038"),
        },
        FirstValueTestCase {
            description: "a bracket array literal builds as ARRAY_CONSTRUCT does",
            sql: "SELECT TO_JSON(['a', 2]) AS j",
            expected_outcome: Ok("[\"a\",2]"),
        },
        FirstValueTestCase {
            description: "an empty bracket array literal has no elements",
            sql: "SELECT ARRAY_SIZE([]) AS n",
            expected_outcome: Ok("0"),
        },
        FirstValueTestCase {
            description: "REDUCE folds a FLOAT accumulator over the elements in order",
            sql: "SELECT REDUCE(PARSE_JSON('[{\"amount\":\"1.5\"},{\"amount\":\"x\"},\
                  {\"amount\":\"2\"}]'), NULL::DOUBLE, (total, item) -> \
                  IFF(TRY_TO_DOUBLE(item:amount::VARCHAR) IS NULL, total, \
                  COALESCE(total, 0) + TRY_TO_DOUBLE(item:amount::VARCHAR))) AS s",
            expected_outcome: Ok("3.5"),
        },
        FirstValueTestCase {
            description: "a FLOAT between NUMBER bounds compares them as doubles past 2^53",
            sql: "SELECT 9007199254740992::FLOAT BETWEEN 9007199254740993 AND 9007199254740993 AS b",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a NUMBER between a FLOAT and a NUMBER bound compares each as the pair does",
            sql: "SELECT CAST(9007199254740993 AS NUMBER(20,0)) NOT BETWEEN 9007199254740992::FLOAT \
                  AND CAST(9007199254740993 AS NUMBER(20,0)) AS b",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "MAX_BY with tied keys holding -0 and 0 raises extreme_key_ties",
            sql: "SELECT MAX_BY(v, k) AS m FROM (SELECT 1 AS k, -0.0::FLOAT AS v \
                  UNION ALL SELECT 1 AS k, 0.0::FLOAT AS v) AS t",
            expected_outcome: Err("extreme_key_ties"),
        },
        FirstValueTestCase {
            description: "MIN_BY with tied keys holding the same zero converts",
            sql: "SELECT MIN_BY(v, k) AS m FROM (SELECT 1 AS k, 0.0::FLOAT AS v \
                  UNION ALL SELECT 1 AS k, 0.0::FLOAT AS v) AS t",
            expected_outcome: Ok("0.0"),
        },
        FirstValueTestCase {
            description: "ANY_VALUE of a group holding -0 and 0 raises any_value_choice",
            sql: "SELECT ANY_VALUE(v) AS a FROM (SELECT -0.0::FLOAT AS v UNION ALL SELECT 0.0::FLOAT AS v) AS t",
            expected_outcome: Err("any_value_choice"),
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
