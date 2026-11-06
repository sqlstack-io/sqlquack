use sqlquack_core::refusals::models::RefusalCategory;

use crate::analysis::tests::helpers::{
    relying_on_utc_session, session_outcome, shop_first_outcome,
};
use crate::analysis::tests::test_types::{
    DivergenceModeOutcome, FirstValueTestCase, SessionTimeZoneTestCase,
};

const NAIVE_TO_ZONED: &str =
    "SELECT '2024-03-10 22:30:00'::TIMESTAMP_NTZ(6)::TIMESTAMP_TZ AS placed_at";

const NAIVE_BESIDE_LOCAL: &str = "SELECT '2024-06-01 10:00:00'::TIMESTAMP_NTZ < \
                                  '2024-06-01 12:00:00 +02:00'::TIMESTAMP_LTZ AS v";

#[test]
fn given_session_dependent_statements_when_converting_then_they_rely_on_a_utc_session() {
    let test_cases = [
        SessionTimeZoneTestCase {
            description: "a naive timestamp read as TIMESTAMP_TZ relies on the UTC session",
            sql: NAIVE_TO_ZONED,
            strict: false,
            utc_session: false,
            expected_outcome: relying_on_utc_session(&["node:cast"]),
        },
        SessionTimeZoneTestCase {
            description: "strict mode refuses it",
            sql: NAIVE_TO_ZONED,
            strict: true,
            utc_session: false,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
        SessionTimeZoneTestCase {
            description: "strict mode converts it once the caller confirms a UTC session",
            sql: NAIVE_TO_ZONED,
            strict: true,
            utc_session: true,
            expected_outcome: relying_on_utc_session(&["node:cast"]),
        },
        SessionTimeZoneTestCase {
            description: "timestamp text without an offset read as TIMESTAMP_TZ relies on it",
            sql: "SELECT '2024-03-10 22:30:00'::TIMESTAMP_TZ AS placed_at",
            strict: false,
            utc_session: false,
            expected_outcome: relying_on_utc_session(&["node:cast"]),
        },
        SessionTimeZoneTestCase {
            description: "timestamp text with an offset does not depend on the session",
            sql: "SELECT '2024-03-10 22:30:00 -05:00'::TIMESTAMP_TZ AS placed_at",
            strict: true,
            utc_session: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![],
                categories: vec![],
            },
        },
        SessionTimeZoneTestCase {
            description: "CONVERT_TIMEZONE of a zoned value does not depend on the session",
            sql: "SELECT CONVERT_TIMEZONE('UTC', '2024-03-10 22:30:00 -05:00'::TIMESTAMP_TZ) \
                  AS placed_at",
            strict: true,
            utc_session: false,
            expected_outcome: DivergenceModeOutcome {
                converted: true,
                divergences: vec![],
                categories: vec![],
            },
        },
        SessionTimeZoneTestCase {
            description: "CONVERT_TIMEZONE with two arguments of a naive value relies on it",
            sql: "SELECT CONVERT_TIMEZONE('UTC', '2024-03-10 22:30:00'::TIMESTAMP_NTZ(6)) \
                  AS placed_at",
            strict: false,
            utc_session: false,
            expected_outcome: relying_on_utc_session(&["fn:CONVERT_TIMEZONE"]),
        },
        SessionTimeZoneTestCase {
            description: "a naive timestamp compared with a local one relies on it",
            sql: NAIVE_BESIDE_LOCAL,
            strict: false,
            utc_session: false,
            expected_outcome: relying_on_utc_session(&["node:lt"]),
        },
        SessionTimeZoneTestCase {
            description: "strict mode refuses that comparison without a confirmed UTC session",
            sql: NAIVE_BESIDE_LOCAL,
            strict: true,
            utc_session: false,
            expected_outcome: DivergenceModeOutcome {
                converted: false,
                divergences: vec![],
                categories: vec![RefusalCategory::DivergenceRefused],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            session_outcome(test_case.sql, (test_case.strict, test_case.utc_session)),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_naive_and_local_timestamps_when_converting_then_they_compare_as_utc_wall_times() {
    let test_cases = [
        FirstValueTestCase {
            description: "a local value at +02:00 equals the naive value of its UTC wall time",
            sql: "SELECT '2024-06-01 10:00:00'::TIMESTAMP_NTZ = \
                  '2024-06-01 12:00:00 +02:00'::TIMESTAMP_LTZ AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "the local side keeps its microseconds beside a millisecond naive value",
            sql: "SELECT '2024-06-01 10:00:00.123'::TIMESTAMP_NTZ(3) < \
                  '2024-06-01 05:00:00.123001 -05:00'::TIMESTAMP_LTZ(9) AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a past naive timestamp is at most CURRENT_TIMESTAMP()",
            sql: "SELECT '2020-01-01 00:00:00'::TIMESTAMP_NTZ <= CURRENT_TIMESTAMP() AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "the incremental filter against DATEADD of CURRENT_TIMESTAMP()",
            sql: "SELECT '2199-01-01 00:00:00'::TIMESTAMP_NTZ >= \
                  DATEADD(day, -3, CURRENT_TIMESTAMP()) AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a naive timestamp four days back is outside a three-day window",
            sql: "SELECT DATEADD(day, -4, CURRENT_TIMESTAMP())::TIMESTAMP_NTZ >= \
                  DATEADD(day, -3, CURRENT_TIMESTAMP()) AS v",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "DATEDIFF in hours reads a DATE as its midnight",
            sql: "SELECT DATEDIFF(hour, '1969-12-31'::DATE, '1970-01-01 01:00:00'::TIMESTAMP_NTZ) \
                  AS v",
            expected_outcome: Ok("25"),
        },
        FirstValueTestCase {
            description: "DATEDIFF in hours to the hour before a DATE's midnight is -1",
            sql: "SELECT DATEDIFF(hour, '2024-06-02'::DATE, '2024-06-01 23:59:59'::TIMESTAMP_NTZ) \
                  AS v",
            expected_outcome: Ok("-1"),
        },
        FirstValueTestCase {
            description: "DATEDIFF in minutes between two DATEs",
            sql: "SELECT DATEDIFF(minute, '2024-06-01'::DATE, '2024-06-02'::DATE) AS v",
            expected_outcome: Ok("1440"),
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
