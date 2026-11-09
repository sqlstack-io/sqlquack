use crate::snowflake::typing::models::SnowflakeType;

use crate::snowflake::types::main::utc_kept::utc_kept;
use crate::snowflake::types::main::zoned_in_utc::zoned_in_utc;
use crate::snowflake::types::tests::test_types::UtcOffsetTestCase;

const ZONED: SnowflakeType = SnowflakeType::TimestampTz { precision: 9 };
const NAIVE: SnowflakeType = SnowflakeType::TimestampNtz { precision: 9 };

#[test]
fn given_chosen_values_when_typing_then_the_utc_offset_is_kept_only_if_all_hold_it() {
    let test_cases = [
        UtcOffsetTestCase {
            description: "UTC-offset values and NULL keep the offset",
            values: vec![(ZONED, true), (SnowflakeType::Null, false)],
            result: ZONED,
            expected_utc_offset: true,
        },
        UtcOffsetTestCase {
            description: "a zoned value of unknown offset loses it",
            values: vec![(ZONED, true), (ZONED, false)],
            result: ZONED,
            expected_utc_offset: false,
        },
        UtcOffsetTestCase {
            description: "a naive value among them loses it",
            values: vec![(ZONED, true), (NAIVE, false)],
            result: ZONED,
            expected_utc_offset: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            utc_kept(&test_case.values, test_case.result),
            test_case.expected_utc_offset,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_casts_to_timestamp_tz_when_typing_then_naive_values_take_the_utc_session_offset() {
    let test_cases = [
        UtcOffsetTestCase {
            description: "a naive timestamp takes the session's +00:00",
            values: vec![(NAIVE, false)],
            result: ZONED,
            expected_utc_offset: true,
        },
        UtcOffsetTestCase {
            description: "a DATE takes the session's +00:00",
            values: vec![(SnowflakeType::Date, false)],
            result: ZONED,
            expected_utc_offset: true,
        },
        UtcOffsetTestCase {
            description: "a zoned value of unknown offset keeps it unknown",
            values: vec![(ZONED, false)],
            result: ZONED,
            expected_utc_offset: false,
        },
        UtcOffsetTestCase {
            description: "a cast to another type holds no offset",
            values: vec![(ZONED, true)],
            result: NAIVE,
            expected_utc_offset: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            zoned_in_utc(test_case.values[0], test_case.result),
            test_case.expected_utc_offset,
            "{}",
            test_case.description
        );
    }
}
