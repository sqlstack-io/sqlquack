use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::FirstValueTestCase;

#[test]
fn given_typed_builds_holding_a_float_when_converting_then_results_leave_as_snowflake_json() {
    let test_cases = [
        FirstValueTestCase {
            description: "an array holding a FLOAT is written with the double as %.15e",
            sql: "SELECT ARRAY_CONSTRUCT(1.5::FLOAT, 2) AS v",
            expected_outcome: Ok("[1.500000000000000e+00,2]"),
        },
        FirstValueTestCase {
            description: "a held object passes through a CTE and leaves as JSON",
            sql: "WITH t AS (SELECT OBJECT_CONSTRUCT('k', 0.5::FLOAT) AS o) SELECT o FROM t",
            expected_outcome: Ok("{\"k\":5.000000000000000e-01}"),
        },
        FirstValueTestCase {
            description: "a held object passes through a star and leaves as JSON",
            sql: "WITH t AS (SELECT OBJECT_CONSTRUCT('k', 0.5::FLOAT) AS o) SELECT * FROM t",
            expected_outcome: Ok("{\"k\":5.000000000000000e-01}"),
        },
        FirstValueTestCase {
            description: "DISTINCT over a held column a star reads is refused",
            sql: "WITH t AS (SELECT ARRAY_CONSTRUCT(0.5::FLOAT) AS v) SELECT DISTINCT * FROM t",
            expected_outcome: Err("a VARIANT built from a FLOAT"),
        },
        FirstValueTestCase {
            description: "a held value from a scalar subquery keeps its type",
            sql: "SELECT (SELECT TO_VARIANT(0.5::FLOAT)) AS v",
            expected_outcome: Ok("5.000000000000000e-01"),
        },
        FirstValueTestCase {
            description: "TO_VARCHAR of a held double raises float_text, as ::VARCHAR does",
            sql: "SELECT TO_VARCHAR(TO_VARIANT(0.5::FLOAT)) AS v",
            expected_outcome: Err("float_text"),
        },
        FirstValueTestCase {
            description: "a held FLOAT that is negative zero raises json_fidelity",
            sql: "SELECT TO_VARIANT(-0.0::FLOAT) AS v",
            expected_outcome: Err("json_fidelity"),
        },
        FirstValueTestCase {
            description: "DISTINCT over a held output is refused",
            sql: "SELECT DISTINCT ARRAY_CONSTRUCT(0.5::FLOAT) AS v",
            expected_outcome: Err("a VARIANT built from a FLOAT"),
        },
        FirstValueTestCase {
            description: "ORDER BY the position of a held output is refused",
            sql: "SELECT TO_VARIANT(0.5::FLOAT) AS v ORDER BY 1",
            expected_outcome: Err("a VARIANT built from a FLOAT"),
        },
        FirstValueTestCase {
            description: "IN over a held subquery column is refused",
            sql: "SELECT PARSE_JSON('1') IN (SELECT TO_VARIANT(0.5::FLOAT)) AS v",
            expected_outcome: Err("a VARIANT built from a FLOAT"),
        },
        FirstValueTestCase {
            description: "an object field holding a FLOAT has the FLOAT's text, as recorded",
            sql: "SELECT OBJECT_CONSTRUCT('k', 0.30000000000000004::FLOAT):k::VARCHAR AS v",
            expected_outcome: Ok("0.3"),
        },
        FirstValueTestCase {
            description: "an array element holding a FLOAT has the FLOAT's text, as recorded",
            sql: "SELECT ARRAY_CONSTRUCT(1e15::FLOAT)[0]::VARCHAR AS v",
            expected_outcome: Ok("1e+15"),
        },
        FirstValueTestCase {
            description: "an element's text keeps the FLOAT's ten significant digits below one, as recorded",
            sql: "SELECT OBJECT_CONSTRUCT('k', 3.141592653589793::FLOAT):k::VARCHAR AS v",
            expected_outcome: Ok("3.141592654"),
        },
        FirstValueTestCase {
            description: "a whole FLOAT element is written without a point, as recorded",
            sql: "SELECT ARRAY_CONSTRUCT(123456789::FLOAT)[0]::VARCHAR AS v",
            expected_outcome: Ok("123456789"),
        },
        FirstValueTestCase {
            description: "an element with two whole digits is written with eleven, as recorded",
            sql: "SELECT OBJECT_CONSTRUCT('k', 12.345678901234567::FLOAT):k::VARCHAR AS v",
            expected_outcome: Ok("12.345678901"),
        },
        FirstValueTestCase {
            description: "a subnormal element has ten significant digits, as recorded",
            sql: "SELECT ARRAY_CONSTRUCT(5e-324::FLOAT)[0]::VARCHAR AS v",
            expected_outcome: Ok("4.940656458e-324"),
        },
        FirstValueTestCase {
            description: "a FLOAT near one is written with ten significant digits, as recorded",
            sql: "SELECT 3.141592653589793::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("3.141592654"),
        },
        FirstValueTestCase {
            description: "a FLOAT below a million is written with fifteen, as recorded",
            sql: "SELECT 123456.78901234567::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("123456.789012346"),
        },
        FirstValueTestCase {
            description: "a FLOAT past 2^53 is written with fifteen, as recorded",
            sql: "SELECT 9007199254740992::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("9.00719925474099e+15"),
        },
        FirstValueTestCase {
            description: "the text of a held double at the root still raises, as Snowflake may fold it",
            sql: "SELECT TO_VARIANT(1e15::FLOAT)::VARCHAR AS v",
            expected_outcome: Err("float_text"),
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
