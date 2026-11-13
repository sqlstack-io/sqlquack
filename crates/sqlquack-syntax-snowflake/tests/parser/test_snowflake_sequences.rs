use crate::helpers::snowflake_kind;
use crate::test_types::SnowflakeVerdictTestCase;

#[test]
fn given_a_snowflake_sequence_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "keeps SET of each property (recorded success)",
            sql: "ALTER SEQUENCE IF EXISTS v1 SET INCREMENT BY = 1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SET of an order and a named comment (recorded semantic error)",
            sql: "ALTER SEQUENCE v1 SET NOORDER COMMENT = m1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "SET needs a property (recorded errno 1003)",
            sql: "ALTER SEQUENCE v1 SET ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps INCREMENT = without SET (recorded semantic error)",
            sql: "ALTER SEQUENCE v1 INCREMENT = 1 ;",
            expected_kind: Some("alter_sequence"),
        },
        SnowflakeVerdictTestCase {
            description: "ALTER SEQUENCE needs an action (recorded errno 1003)",
            sql: "ALTER SEQUENCE IF EXISTS v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps the smallest START (recorded success)",
            sql: "CREATE SEQUENCE v1 START -9223372036854775808 ;",
            expected_kind: Some("create_sequence"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            snowflake_kind(test_case.sql),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
