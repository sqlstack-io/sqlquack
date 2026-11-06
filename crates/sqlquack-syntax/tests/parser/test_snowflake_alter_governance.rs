use crate::helpers::snowflake_kind;
use crate::test_types::SnowflakeVerdictTestCase;

#[test]
fn given_snowflake_alter_governance_actions_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a column masking policy with FORCE (recorded OK)",
            sql: "ALTER TABLE v1 ALTER COLUMN v1 SET MASKING POLICY v1 USING ( v1 , v1 ) FORCE ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "tags on two columns of a view (recorded OK)",
            sql: "ALTER VIEW v1 ALTER v1 SET TAG v1 = 'm' , v1 SET TAG v1 = 'm' , v1 = 'm' ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "an added column with a masking policy (recorded OK)",
            sql: "ALTER ICEBERG TABLE v1 ADD v1 INT WITH MASKING POLICY v1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "an aggregation policy with an entity key on a view (recorded OK)",
            sql: "ALTER VIEW v1 SET AGGREGATION POLICY v1 ENTITY KEY ( v1 , v1 ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a row access policy swap onto no columns (recorded OK)",
            sql: "ALTER TABLE v1 DROP ROW ACCESS POLICY v1 , ADD ROW ACCESS POLICY v1 ON ( ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a contact purpose on a view (recorded OK)",
            sql: "ALTER VIEW v1 SET CONTACT STEWARD = v1 , STEWARD = v1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a bare JOIN property after UNSET (recorded OK)",
            sql: "ALTER TABLE v1 UNSET JOIN ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a plain property SET stays with the table parser (recorded OK)",
            sql: "ALTER TABLE v1 SET v1 = 'm' ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "an aggregation policy named by a string (recorded errno 1003)",
            sql: "ALTER TABLE v1 SET AGGREGATION POLICY 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET POLICY with no policy kind (recorded errno 1003)",
            sql: "ALTER TABLE v1 SET POLICY v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "DROP ROW ACCESS POLICIES without ALL (recorded errno 1003)",
            sql: "ALTER TABLE v1 DROP ROW ACCESS POLICIES ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a contact named by a string (recorded errno 1003)",
            sql: "ALTER TABLE v1 SET CONTACT STEWARD = 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a contact purpose on a materialized view (recorded errno 1003)",
            sql: "ALTER MATERIALIZED VIEW v1 UNSET CONTACT STEWARD ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a column UNSET TAG with no tag (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER v1 UNSET TAG ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a second column item with a plain property (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER v1 SET TAG v1 = 'm' , v1 SET v1 = 'm' ;",
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
