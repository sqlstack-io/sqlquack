use sqlquack_core::refusals::models::RefusalCategory;

use crate::analysis::tests::helpers::{relying_on_utc_session, session_outcome};
use crate::analysis::tests::test_types::{DivergenceModeOutcome, SessionTimeZoneTestCase};

const NAIVE_TO_ZONED: &str =
    "SELECT '2024-03-10 22:30:00'::TIMESTAMP_NTZ(6)::TIMESTAMP_TZ AS placed_at";

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
                categories: vec![RefusalCategory::TypeCondition],
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
