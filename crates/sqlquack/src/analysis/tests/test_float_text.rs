use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::FirstValueTestCase;

#[test]
fn given_a_float_when_written_as_text_then_its_digits_follow_the_recorded_bands() {
    let test_cases = [
        FirstValueTestCase {
            description: "below 0.1, rounding onto it, with ten digits",
            sql: "SELECT 0.09999999999999999::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("0.1"),
        },
        FirstValueTestCase {
            description: "below 10, rounding onto it with ten digits",
            sql: "SELECT 9.9999999999::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("10"),
        },
        FirstValueTestCase {
            description: "just below 10",
            sql: "SELECT 9.999999999999998::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("10"),
        },
        FirstValueTestCase {
            description: "just above 10, with eleven digits",
            sql: "SELECT 10.000000000000002::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("10"),
        },
        FirstValueTestCase {
            description: "two whole digits, with eleven",
            sql: "SELECT 12.345678901234567::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("12.345678901"),
        },
        FirstValueTestCase {
            description: "below 100, rounding onto it",
            sql: "SELECT 99.99999999999::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("100"),
        },
        FirstValueTestCase {
            description: "four whole digits, with thirteen",
            sql: "SELECT 1234.5678901234567::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("1234.567890123"),
        },
        FirstValueTestCase {
            description: "below 1e5, rounding onto it with fourteen",
            sql: "SELECT 99999.99999999999::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("100000"),
        },
        FirstValueTestCase {
            description: "from 1e5, with fifteen",
            sql: "SELECT 123456.78901234567::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("123456.789012346"),
        },
        FirstValueTestCase {
            description: "seven whole digits, capped at fifteen",
            sql: "SELECT 1234567.8901234567::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("1234567.89012346"),
        },
        FirstValueTestCase {
            description: "fifteen whole digits, still plain",
            sql: "SELECT 123456789012345::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("123456789012345"),
        },
        FirstValueTestCase {
            description: "rounding onto 1e15, which switches to an exponent",
            sql: "SELECT 999999999999999.9::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("1e+15"),
        },
        FirstValueTestCase {
            description: "1e15, the first exponent above",
            sql: "SELECT 1e15::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("1e+15"),
        },
        FirstValueTestCase {
            description: "0.0001, the last plain below",
            sql: "SELECT 0.0001::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("0.0001"),
        },
        FirstValueTestCase {
            description: "1e-5, the first exponent below",
            sql: "SELECT 1e-5::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("1e-05"),
        },
        FirstValueTestCase {
            description: "a negative with five whole digits",
            sql: "SELECT (-12345.678901234567)::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("-12345.678901235"),
        },
        FirstValueTestCase {
            description: "the smallest subnormal",
            sql: "SELECT 5e-324::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("4.940656458e-324"),
        },
        FirstValueTestCase {
            description: "the largest double",
            sql: "SELECT 1.7976931348623157e308::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("1.79769313486232e+308"),
        },
        FirstValueTestCase {
            description: "TO_VARCHAR writes the same digits as ::VARCHAR",
            sql: "SELECT TO_VARCHAR(1234567.8901234567::FLOAT) AS v",
            expected_outcome: Ok("1234567.89012346"),
        },
        FirstValueTestCase {
            description: "a FLOAT read by a path out of a VARIANT writes the same digits",
            sql: "SELECT ARRAY_CONSTRUCT(12345.678901234567::FLOAT)[0]::VARCHAR AS v",
            expected_outcome: Ok("12345.678901235"),
        },
        FirstValueTestCase {
            description: "negative zero is written -0, as recorded over many rows",
            sql: "SELECT '-0'::FLOAT::VARCHAR AS v",
            expected_outcome: Ok("-0"),
        },
        FirstValueTestCase {
            description: "a FLOAT at a VARIANT's root still raises float_text, as Snowflake may fold it",
            sql: "SELECT TO_VARIANT(12.5::FLOAT)::VARCHAR AS v",
            expected_outcome: Err("float_text"),
        },
        FirstValueTestCase {
            description: "a FLOAT concatenated with text is written as ::VARCHAR writes it",
            sql: "SELECT 'v=' || 12.345678901234567::FLOAT AS v",
            expected_outcome: Ok("v=12.345678901"),
        },
        FirstValueTestCase {
            description: "CONCAT and CONCAT_WS write a FLOAT as ::VARCHAR writes it",
            sql: "SELECT CONCAT_WS(',', 1e15::FLOAT, CONCAT('NaN'::FLOAT, '|')) AS v",
            expected_outcome: Ok("1e+15,NaN|"),
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
