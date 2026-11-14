use sqlquack_syntax::Dialect;

use crate::helpers::accepted;
use crate::test_types::AcceptanceTestCase;

#[test]
fn given_a_snowflake_option_whose_value_is_a_string_when_parsing_then_a_bare_word_is_refused() {
    let test_cases = [
        AcceptanceTestCase {
            description: "an agent's live version specification as a string (recorded errno 2003)",
            sql: "ALTER AGENT v1 MODIFY LIVE VERSION SET SPECIFICATION = '{}' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an agent's live version specification as a bare word (recorded errno 1003)",
            sql: "ALTER AGENT v1 MODIFY LIVE VERSION SET SPECIFICATION = m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an agent's live version specification with no value (recorded errno 1003)",
            sql: "ALTER AGENT v1 MODIFY LIVE VERSION SET SPECIFICATION = ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a scoring profile as a string (recorded errno 2003)",
            sql: "ALTER CORTEX SEARCH SERVICE v1 ADD SCORING PROFILE v1 '{}' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a scoring profile as a bare word (recorded errno 1003)",
            sql: "ALTER CORTEX SEARCH SERVICE v1 ADD SCORING PROFILE v1 m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a scoring profile with no profile (recorded errno 1003)",
            sql: "ALTER CORTEX SEARCH SERVICE v1 ADD SCORING PROFILE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a failover group's schedule as a bare word (recorded errno 1003)",
            sql: "CREATE FAILOVER GROUP v1 OBJECT_TYPES = DATABASES ALLOWED_ACCOUNTS = v1 . v1 REPLICATION_SCHEDULE = m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a replication group's schedule as a bare word (recorded errno 1003)",
            sql: "CREATE REPLICATION GROUP v1 OBJECT_TYPES = DATABASES ALLOWED_ACCOUNTS = v1 . v1 REPLICATION_SCHEDULE = m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a failover group's schedule as a string",
            sql: "CREATE FAILOVER GROUP v1 OBJECT_TYPES = DATABASES ALLOWED_ACCOUNTS = v1 . v1 REPLICATION_SCHEDULE = '10 MINUTE' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            accepted(test_case.sql, test_case.dialect),
            test_case.expected_accepted,
            "{}",
            test_case.description
        );
    }
}
