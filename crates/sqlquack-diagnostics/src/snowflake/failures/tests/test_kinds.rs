use crate::failures::constants::NUMBER_OUT_OF_RANGE;
use crate::failures::models::ErrorKind;
use crate::snowflake::failures::main::snowflake_kind::snowflake_kind;
use crate::snowflake::failures::tests::test_types::SnowflakeErrnoTestCase;

#[test]
fn given_snowflake_errno_when_classifying_then_errno_and_sqlstate_give_kind() {
    let test_cases = [
        SnowflakeErrnoTestCase {
            description: "numeric overflow",
            errno: NUMBER_OUT_OF_RANGE.errno,
            sqlstate: NUMBER_OUT_OF_RANGE.sqlstate,
            expected_kind: Some(ErrorKind::NumericOverflow),
        },
        SnowflakeErrnoTestCase {
            description: "a known number with another SQLSTATE",
            errno: NUMBER_OUT_OF_RANGE.errno,
            sqlstate: "22000",
            expected_kind: None,
        },
        SnowflakeErrnoTestCase {
            description: "an unknown error",
            errno: 1,
            sqlstate: "00000",
            expected_kind: None,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            snowflake_kind(test_case.errno, test_case.sqlstate),
            test_case.expected_kind,
            "{}",
            test_case.description
        );
    }
}
