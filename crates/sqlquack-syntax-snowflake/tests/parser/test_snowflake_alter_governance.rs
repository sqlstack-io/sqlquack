use serde_json::json;

use crate::helpers::snowflake_kind;
use crate::helpers::statements;
use crate::test_types::{ProjectionTestCase, SnowflakeVerdictTestCase};
use sqlquack_syntax::{CallMode, Dialect};

#[test]
fn given_snowflake_alter_governance_actions_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a column masking policy with FORCE (recorded OK)",
            sql: "ALTER TABLE v1 ALTER COLUMN v1 SET MASKING POLICY v1 USING ( v1 , v1 ) FORCE ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "tags on two columns of a view (recorded OK)",
            sql: "ALTER VIEW v1 ALTER v1 SET TAG v1 = 'm' , v1 SET TAG v1 = 'm' , v1 = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an added column with a masking policy is an added column (recorded OK)",
            sql: "ALTER ICEBERG TABLE v1 ADD v1 INT WITH MASKING POLICY v1 ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "an aggregation policy with an entity key on a view (recorded OK)",
            sql: "ALTER VIEW v1 SET AGGREGATION POLICY v1 ENTITY KEY ( v1 , v1 ) ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a row access policy swap onto no columns keeps both policies (recorded OK)",
            sql: "ALTER TABLE v1 DROP ROW ACCESS POLICY v1 , ADD ROW ACCESS POLICY v1 ON ( ) ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a contact purpose on a view (recorded OK)",
            sql: "ALTER VIEW v1 SET CONTACT STEWARD = v1 , STEWARD = v1 ;",
            expected_kind: Some("alter_object"),
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
            expected_kind: Some("alter_object"),
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
            expected_kind: Some("alter_object"),
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

#[test]
fn given_a_snowflake_column_governance_action_when_parsing_then_each_column_is_a_property() {
    let test_cases = [
        ProjectionTestCase {
            description: "the statement alters a table",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/kind",
            expected_value: json!("TABLE"),
        },
        ProjectionTestCase {
            description: "the action is ALTER COLUMN",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/action",
            expected_value: json!("ALTER COLUMN"),
        },
        ProjectionTestCase {
            description: "the first column sets a masking policy",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/properties/0/kind",
            expected_value: json!("SET MASKING POLICY"),
        },
        ProjectionTestCase {
            description: "an aggregation policy's ENTITY KEY is its own property",
            sql: "ALTER TABLE v1 SET AGGREGATION POLICY p1 ENTITY KEY ( c1 ) FORCE ;",
            pointer: "/0/alter_object/properties/1/kind",
            expected_value: json!("ENTITY KEY"),
        },
        ProjectionTestCase {
            description: "FORCE follows the ENTITY KEY",
            sql: "ALTER TABLE v1 SET AGGREGATION POLICY p1 ENTITY KEY ( c1 ) FORCE ;",
            pointer: "/0/alter_object/properties/2/kind",
            expected_value: json!("FORCE"),
        },
        ProjectionTestCase {
            description: "UNSET JOIN without POLICY invents no POLICY",
            sql: "ALTER VIEW v1 UNSET JOIN ;",
            pointer: "/0/alter_object/properties/0/kind",
            expected_value: json!("JOIN"),
        },
        ProjectionTestCase {
            description: "MODIFY without COLUMN records no COLUMN",
            sql: "ALTER TABLE v1 MODIFY v1 SET MASKING POLICY p1 ;",
            pointer: "/0/alter_object/properties/0/column_keyword",
            expected_value: json!(null),
        },
        ProjectionTestCase {
            description: "an added column's WITH TAG keeps its WITH",
            sql: "ALTER TABLE v1 ADD v1 INT WITH TAG ( t1 = 'm' ) ;",
            pointer: "/0/alter_table/actions/0/AddColumn/column/constraints/0/Tags/with",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a column's WITH PROJECTION POLICY keeps its WITH",
            sql: "CREATE TABLE v1 ( v1 INT WITH PROJECTION POLICY p1 ) ;",
            pointer: "/0/create_object/operands/0/tuple/expressions/0/column_def/options/0/property_e_q/left/var/this",
            expected_value: json!("WITH PROJECTION POLICY"),
        },
        ProjectionTestCase {
            description: "the first column keeps its name",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/properties/0/name/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "FORCE is its own property of the first column",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/properties/1",
            expected_value: json!({"kind": "FORCE", "name": {"name": "v1", "quoted": false, "trailing_comments": [], "span": {"start": 21, "end": 23, "line": 1, "column": 24}}, "value": null}),
        },
        ProjectionTestCase {
            description: "the second column unsets tags and keeps its written COLUMN",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/properties/2/column_keyword",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "the second column's action",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/properties/2/kind",
            expected_value: json!("UNSET TAG"),
        },
        ProjectionTestCase {
            description: "there are only two columns and one FORCE",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/properties/3",
            expected_value: json!(null),
        },
        ProjectionTestCase {
            description: "the second tag is kept",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/properties/2/value/tuple/expressions/1/table/name/name",
            expected_value: json!("t2"),
        },
        ProjectionTestCase {
            description: "there are only two tags",
            sql: "ALTER TABLE v1 ALTER v1 SET MASKING POLICY p1 USING ( v1 , v2 ) FORCE , COLUMN v2 UNSET TAG t1 , t2 ;",
            pointer: "/0/alter_object/properties/2/value/tuple/expressions/2",
            expected_value: json!(null),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            statements(test_case.sql, Dialect::Snowflake, CallMode::Faithful)
                .pointer(test_case.pointer)
                .cloned()
                .unwrap_or_default(),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
