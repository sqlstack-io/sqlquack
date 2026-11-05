use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::FirstValueTestCase;

#[test]
fn given_utc_offset_values_when_carried_through_queries_then_their_wall_times_convert() {
    let test_cases = [
        FirstValueTestCase {
            description: "a UTC-offset value through a CTE casts to its UTC wall time",
            sql: "WITH z AS (SELECT CONVERT_TIMEZONE('UTC', \
                  '2024-03-10 22:30:00 -05:00'::TIMESTAMP_TZ) AS u) \
                  SELECT CAST(u AS DATE)::VARCHAR AS v FROM z",
            expected_outcome: Ok("2024-03-11"),
        },
        FirstValueTestCase {
            description: "DATE_TRUNC of a UTC-offset value through MIN and a CTE",
            sql: "WITH z AS (SELECT MIN(CONVERT_TIMEZONE('UTC', \
                  '2024-03-10 22:30:00 -05:00'::TIMESTAMP_TZ)) AS u) \
                  SELECT CAST(DATE_TRUNC('month', u) AS DATE)::VARCHAR AS v FROM z",
            expected_outcome: Ok("2024-03-01"),
        },
        FirstValueTestCase {
            description: "DATEDIFF of two naive values cast to TIMESTAMP_TZ in the UTC session",
            sql: "WITH z AS (SELECT '2024-03-10 22:30:00'::TIMESTAMP_NTZ::TIMESTAMP_TZ AS a, \
                  '2024-03-11 01:00:00'::TIMESTAMP_NTZ::TIMESTAMP_TZ AS b) \
                  SELECT DATEDIFF('day', a, b) AS v FROM z",
            expected_outcome: Ok("1"),
        },
        FirstValueTestCase {
            description: "a UTC-offset value in an object is written with a trailing Z, as recorded",
            sql: "SELECT OBJECT_CONSTRUCT_KEEP_NULL('at', CONVERT_TIMEZONE('UTC', \
                  '2024-03-10 02:30:00 -05:00'::TIMESTAMP_TZ))::VARCHAR AS v",
            expected_outcome: Ok("{\"at\":\"2024-03-10 07:30:00.000 Z\"}"),
        },
        FirstValueTestCase {
            description: "the target spelled 'utc', which Snowflake rejects, is refused",
            sql: "SELECT CAST(CONVERT_TIMEZONE('utc', \
                  '2024-03-10 22:30:00 -05:00'::TIMESTAMP_TZ) AS TIMESTAMP_NTZ) AS v",
            expected_outcome: Err("no recording shows Snowflake accepting"),
        },
        FirstValueTestCase {
            description: "a zoned value of unknown offset is still refused",
            sql: "SELECT CAST('2024-03-10 22:30:00 -05:00'::TIMESTAMP_TZ AS TIMESTAMP_NTZ) AS v",
            expected_outcome: Err("reads the local time of an offset"),
        },
        FirstValueTestCase {
            description: "COALESCE with a zoned value of unknown offset loses the UTC offset",
            sql: "SELECT CAST(COALESCE(CONVERT_TIMEZONE('UTC', \
                  '2024-03-10 22:30:00 -05:00'::TIMESTAMP_TZ), \
                  '2024-03-10 22:30:00 -05:00'::TIMESTAMP_TZ) AS TIMESTAMP_NTZ) AS v",
            expected_outcome: Err("reads the local time of an offset"),
        },
        FirstValueTestCase {
            description: "DATEDIFF of a UTC-offset value against a naive one is refused",
            sql: "SELECT DATEDIFF('hour', CONVERT_TIMEZONE('UTC', \
                  '2024-03-10 22:30:00 -05:00'::TIMESTAMP_TZ), \
                  '2024-03-11 01:00:00'::TIMESTAMP_NTZ) AS v",
            expected_outcome: Err("another kind"),
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
