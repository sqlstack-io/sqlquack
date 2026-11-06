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
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a contact purpose on a view (recorded OK)",
            sql: "ALTER VIEW v1 SET CONTACT STEWARD = v1 , STEWARD = v1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a bare JOIN property after UNSET (recorded OK)",
            sql: "ALTER TABLE v1 UNSET JOIN ;",
            expected_kind: Some("alter_table"),
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
            description: "a column UNSET TAG named with COLUMN (recorded OK)",
            sql: "ALTER TABLE v1 ALTER COLUMN v1 UNSET TAG v1 , v1 , COLUMN v1 UNSET TAG v1 , v1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a column UNSET TAG without COLUMN (recorded errno 1003)",
            sql: "ALTER VIEW v1 MODIFY v1 UNSET TAG v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a column UNSET MASKING POLICY without COLUMN (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER v1 UNSET MASKING POLICY ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a column UNSET PROJECTION POLICY without COLUMN (recorded OK)",
            sql: "ALTER TABLE v1 ALTER v1 UNSET PROJECTION POLICY ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "an added column with a masking policy, a projection policy and tags (recorded errno 1003)",
            sql: "ALTER TABLE IF EXISTS v1 ADD COLUMN IF NOT EXISTS v1 INT WITH MASKING POLICY v1 USING ( v1 , v1 ) WITH PROJECTION POLICY v1 WITH TAG ( v1 = 'm' , v1 = 'm' ) ;",
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

#[test]
fn given_a_parenthesized_snowflake_alter_column_list_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a parenthesized list of column actions (recorded OK)",
            sql: "ALTER TABLE v1 ALTER ( v1 DROP DEFAULT , v1 SET DEFAULT v1 . NEXTVAL , v1 NOT NULL , v1 INT , v1 COMMENT 'm' , v1 UNSET COMMENT ) ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a parenthesized list left open (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER ( v1 DROP DEFAULT , v1 SET DEFAULT v1 . NEXTVAL , v1 NOT NULL , v1 INT , v1 COMMENT 'm' , v1 UNSET COMMENT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET TYPE without DATA inside the list (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER ( v1 DROP DEFAULT , v1 SET DEFAULT v1 . NEXTVAL , v1 NOT NULL , v1 SET TYPE INT , v1 COMMENT 'm' , v1 UNSET COMMENT ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a default without SET (recorded OK)",
            sql: "ALTER TABLE v1 ALTER v1 DROP DEFAULT , v1 DEFAULT v1 . NEXTVAL , v1 NOT NULL , v1 INT , v1 COMMENT 'm' , v1 UNSET COMMENT ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a default with no value (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER v1 DEFAULT , v1 SET DEFAULT v1 . NEXTVAL , v1 NOT NULL , v1 INT , v1 COMMENT 'm' , v1 UNSET COMMENT ;",
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
