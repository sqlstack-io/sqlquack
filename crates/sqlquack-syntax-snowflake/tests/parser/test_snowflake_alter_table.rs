use crate::helpers::snowflake_kind;
use crate::test_types::SnowflakeVerdictTestCase;

#[test]
fn given_snowflake_alter_table_add_when_parsing_then_the_table_grammar_judges_it() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an added column's constraint takes its properties (recorded OK)",
            sql: "ALTER TABLE v1 ADD COLUMN v1 INT NOT NULL CONSTRAINT v1 UNIQUE NOT ENFORCED NOT DEFERRABLE INITIALLY DEFERRED ENABLE VALIDATE RELY ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "an out-of-line constraint takes its properties and a comment (recorded OK)",
            sql: "ALTER TABLE v1 ADD CONSTRAINT v1 UNIQUE ( v1 , v1 ) NOT ENFORCED NOT DEFERRABLE INITIALLY DEFERRED ENABLE VALIDATE RELY COMMENT 'm' ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a CHECK constraint takes ENABLE NOVALIDATE (recorded OK)",
            sql: "ALTER TABLE v1 ADD CHECK ( 1 ) ENABLE NOVALIDATE ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a parenthesised column list after ADD (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a constraint without ADD (recorded errno 1003)",
            sql: "ALTER TABLE v1 UNIQUE ( v1 ) ;",
            expected_kind: None,
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
