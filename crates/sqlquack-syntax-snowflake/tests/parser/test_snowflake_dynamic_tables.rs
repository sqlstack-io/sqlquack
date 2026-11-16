use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{snowflake_kind, statements};
use crate::test_types::{ProjectionTestCase, SnowflakeVerdictTestCase};

#[test]
fn given_a_snowflake_alter_dynamic_table_when_parsing_then_each_action_is_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "ALTER DYNAMIC without TABLE (recorded errno 1003)",
            sql: "ALTER DYNAMIC v1 SUSPEND ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "no action (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET with no property (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string contact (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET CONTACT ( STEWARD = 'v1' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "FROZEN without WHERE (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET FROZEN ( 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string policy (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 ADD ROW ACCESS POLICY 'v1' ON ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty clustering key (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 CLUSTER BY ( ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string search target (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 RESUME SEARCH OPTIMIZATION ON 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a second UNSET TAG with no column (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 ALTER v1 UNSET TAG v1 , UNSET TAG v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a column comment that is not a string (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 ALTER v1 COMMENT m1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a name list on another action (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 , v1 SUSPEND ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER DYNAMIC TABLE IF EXISTS v1 , v1 REFRESH COPY SESSION ;",
            sql: "ALTER DYNAMIC TABLE IF EXISTS v1 , v1 REFRESH COPY SESSION ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a parenthesized contact list after another property (recorded errno 1003 for SET CONTACT ( ... ) alone)",
            sql: "ALTER DYNAMIC TABLE v1 SET TARGET_LAG = DOWNSTREAM CONTACT ( STEWARD = v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a parenthesized contact list (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET CONTACT ( STEWARD = v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SUSPEND SEARCH OPTIMIZATION takes no targets (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SUSPEND SEARCH OPTIMIZATION ON EQUALITY ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "RESUME SEARCH OPTIMIZATION takes no targets (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 RESUME SEARCH OPTIMIZATION ON v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SUSPEND SEARCH OPTIMIZATION alone (recorded parse)",
            sql: "ALTER DYNAMIC TABLE v1 SUSPEND SEARCH OPTIMIZATION ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "DROP SEARCH OPTIMIZATION keeps its targets (recorded parse)",
            sql: "ALTER DYNAMIC TABLE v1 DROP SEARCH OPTIMIZATION ON 1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a column that unsets a tag needs COLUMN (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 ALTER v1 UNSET TAG v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "COLUMN before a column that unsets a tag (recorded parse)",
            sql: "ALTER DYNAMIC TABLE v1 ALTER COLUMN v1 UNSET TAG v1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a column sets a tag without COLUMN (recorded parse)",
            sql: "ALTER DYNAMIC TABLE v1 ALTER v1 SET TAG v1 = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "the user's secondary roles as a list (recorded parse)",
            sql: "ALTER DYNAMIC TABLE v1 SET EXECUTE AS USER v1 USE SECONDARY ROLES v1 , v1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "no secondary roles (recorded parse)",
            sql: "ALTER DYNAMIC TABLE v1 SET EXECUTE AS USER v1 USE SECONDARY ROLES NONE ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "USE SECONDARY ROLES ends the SET list (the recorded every-option SET is errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET EXECUTE AS USER v1 USE SECONDARY ROLES ALL ROW_TIMESTAMP = TRUE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "USE SECONDARY ROLES needs its roles (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET EXECUTE AS USER v1 USE SECONDARY ROLES ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a created dynamic table's secondary roles as a list (recorded parse)",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT ) TARGET_LAG = '1 minute' WAREHOUSE = v1 EXECUTE AS USER v1 USE SECONDARY ROLES v1 , v1 AS SELECT 1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER DYNAMIC TABLE v1 UNSET FROZEN WHERE , EXECUTE AS USER ;",
            sql: "ALTER DYNAMIC TABLE v1 UNSET FROZEN WHERE , EXECUTE AS USER ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER DYNAMIC TABLE v1 DROP ROW ACCESS POLICY v1 , ADD ROW ACCESS POLI",
            sql: "ALTER DYNAMIC TABLE v1 DROP ROW ACCESS POLICY v1 , ADD ROW ACCESS POLICY v1 ON ( v1 , v1 ) ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER DYNAMIC TABLE v1 SET AGGREGATION POLICY v1 ENTITY KEY ( v1 ) FOR",
            sql: "ALTER DYNAMIC TABLE v1 SET AGGREGATION POLICY v1 ENTITY KEY ( v1 ) FORCE ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER DYNAMIC TABLE v1 DROP SEARCH OPTIMIZATION ON EQUALITY ( v1 ) , v",
            sql: "ALTER DYNAMIC TABLE v1 DROP SEARCH OPTIMIZATION ON EQUALITY ( v1 ) , v1 , 1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER DYNAMIC TABLE v1 ALTER v1 SET TAG v1 = 'm' , v2 = 'm' , COLUMN v",
            sql: "ALTER DYNAMIC TABLE v1 ALTER v1 SET TAG v1 = 'm' , v2 = 'm' , COLUMN v3 SET TAG v1 = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER DYNAMIC TABLE v1 MODIFY COLUMN v1 UNSET TAG a.b , v2 UNSET COMMENT ;",
            sql: "ALTER DYNAMIC TABLE v1 MODIFY COLUMN v1 UNSET TAG a.b , v2 UNSET COMMENT ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER DYNAMIC TABLE v1 ALTER v1 SET MASKING POLICY v1 USING ( v1 , v2 ",
            sql: "ALTER DYNAMIC TABLE v1 ALTER v1 SET MASKING POLICY v1 USING ( v1 , v2 ) FORCE ;",
            expected_kind: Some("alter_object"),
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
fn given_a_snowflake_dynamic_iceberg_or_interactive_table_when_parsing_then_recorded_verdicts_hold()
{
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an untyped column, quoted warehouse and AS query (recorded OK)",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 ) TARGET_LAG = '1 minute' WAREHOUSE = \"SELECT\" AS SELECT 1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "REFRESH USING an INSERT INTO SELF (recorded OK)",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT ) TARGET_LAG = DOWNSTREAM WAREHOUSE = v1 REFRESH USING ( INSERT INTO SELF SELECT 1 ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a row access policy on no columns (recorded OK)",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT ) TARGET_LAG = '1 minute' WAREHOUSE = v1 ROW ACCESS POLICY v1 ON ( ) AS SELECT 1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "secondary roles ALL (recorded OK)",
            sql: "CREATE OR ALTER DYNAMIC ICEBERG TABLE v1 ( v1 INT ) TARGET_LAG = '1 minute' WAREHOUSE = v1 EXECUTE AS USER v1 USE SECONDARY ROLES ALL AS SELECT 1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "no AS query (recorded errno 1003)",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT ) TARGET_LAG = '1 minute' WAREHOUSE = v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a property with no value before the next (recorded errno 1003)",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT ) TARGET_LAG = WAREHOUSE = v1 AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a storage lifecycle policy on no columns (recorded errno 1003)",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT ) TARGET_LAG = '1 minute' WAREHOUSE = v1 STORAGE LIFECYCLE POLICY v1 ON ( ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "PARTITION BY (recorded errno 1003)",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT ) TARGET_LAG = '1 minute' WAREHOUSE = v1 PARTITION BY ( DAY ( 'v1' ) ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string column type (recorded errno 1003)",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 'INT' ) TARGET_LAG = '1 minute' WAREHOUSE = v1 AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an interactive table with a lag but no warehouse (recorded OK)",
            sql: "CREATE INTERACTIVE TABLE v1 ( v1 INT ) CLUSTER BY ( 1 ) TARGET_LAG = '1 minute' AS SELECT 1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an interactive table with a storage lifecycle policy (recorded OK)",
            sql: "CREATE INTERACTIVE TABLE v1 ( v1 INT ) CLUSTER BY ( 1 ) WITH STORAGE LIFECYCLE POLICY v1 ON ( v1 ) AS SELECT 1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an interactive table without CLUSTER BY (recorded errno 1003)",
            sql: "CREATE INTERACTIVE TABLE v1 ( v1 INT ) BY ( 1 ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an interactive table with a query but no AS (recorded errno 1003)",
            sql: "CREATE INTERACTIVE TABLE v1 ( v1 INT ) CLUSTER BY ( 1 ) SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an interactive table with a row access policy on no columns (recorded errno 1003)",
            sql: "CREATE INTERACTIVE TABLE v1 ( v1 INT ) CLUSTER BY ( 1 ) ROW ACCESS POLICY v1 ON ( ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "INTERACTIVE without TABLE (recorded errno 1003)",
            sql: "CREATE INTERACTIVE v1 CLUSTER BY ( 1 ) AS SELECT 1 ;",
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
fn given_a_snowflake_dynamic_table_clone_or_set_when_parsing_then_the_probed_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a dynamic table clone copies grants (recorded semantic error)",
            sql: "CREATE DYNAMIC TABLE v1 CLONE v1 COPY GRANTS ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a dynamic table clone copies no tags (recorded errno 1003)",
            sql: "CREATE DYNAMIC TABLE v1 CLONE v1 COPY TAGS ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a dynamic table clone takes a COPY list (recorded semantic error)",
            sql: "CREATE DYNAMIC TABLE v1 CLONE v1 COPY = ( COMMENTS ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SET EXECUTE AS USER with secondary roles alone (recorded semantic error)",
            sql: "ALTER DYNAMIC TABLE v1 SET EXECUTE AS USER v1 USE SECONDARY ROLES ALL ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "EXECUTE AS USER after another property (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET TARGET_LAG = '1 minute' EXECUTE AS USER v1 USE SECONDARY ROLES ALL ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a property after EXECUTE AS USER (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET EXECUTE AS USER v1 ROW_TIMESTAMP = TRUE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "FROZEN WHERE after another property (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET TARGET_LAG = '1 minute' FROZEN WHERE ( 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a property after FROZEN WHERE (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 SET FROZEN WHERE ( 1 ) ROW_TIMESTAMP = TRUE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ALTER of a column without COLUMN unsets no masking policy (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 ALTER v1 UNSET MASKING POLICY ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "MODIFY of a column without COLUMN unsets a projection policy (recorded semantic error)",
            sql: "ALTER DYNAMIC TABLE v1 MODIFY v1 UNSET PROJECTION POLICY ;",
            expected_kind: Some("alter_object"),
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
fn given_a_snowflake_dynamic_table_user_with_secondary_roles_when_parsing_then_the_roles_are_their_own_property(
) {
    let test_cases = [
        ProjectionTestCase {
            description: "ALTER SET keeps the user apart from the roles",
            sql: "ALTER DYNAMIC TABLE v1 SET EXECUTE AS USER u1 USE SECONDARY ROLES r1 , r2 ;",
            pointer: "/0/alter_object/properties/0/value/table/name/name",
            expected_value: json!("u1"),
        },
        ProjectionTestCase {
            description: "ALTER SET lists the roles under USE SECONDARY ROLES",
            sql: "ALTER DYNAMIC TABLE v1 SET EXECUTE AS USER u1 USE SECONDARY ROLES r1 , r2 ;",
            pointer: "/0/alter_object/properties/1/value/tuple/expressions/1/table/name/name",
            expected_value: json!("r2"),
        },
        ProjectionTestCase {
            description: "NONE is a keyword, not a role name",
            sql: "CREATE OR ALTER DYNAMIC TABLE v1 TARGET_LAG = '1 minute' WAREHOUSE = v1 EXECUTE AS USER u1 USE SECONDARY ROLES NONE AS SELECT 1 ;",
            pointer: "/0/create_object/properties/3/value/var/this",
            expected_value: json!("NONE"),
        },
        ProjectionTestCase {
            description: "a dynamic iceberg table names the property",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT ) TARGET_LAG = '1 minute' WAREHOUSE = v1 EXECUTE AS USER u1 USE SECONDARY ROLES r1 , r2 AS SELECT 1 ;",
            pointer: "/0/create_object/properties/3/kind",
            expected_value: json!("USE SECONDARY ROLES"),
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

#[test]
fn given_optional_words_in_table_and_index_ddl_when_parsing_then_they_are_kept() {
    let test_cases = [
        ProjectionTestCase {
            description: "FORCE after a dynamic table column's masking policy is kept",
            sql: "ALTER DYNAMIC TABLE v1 ALTER v1 SET MASKING POLICY v1 FORCE ;",
            pointer: "/0/alter_object/operands/0/tuple/expressions/3/identifier/name",
            expected_value: json!("FORCE"),
        },
        ProjectionTestCase {
            description: "a dynamic table's ADD after DROP ROW ACCESS POLICY is its own property",
            sql: "ALTER DYNAMIC TABLE v1 DROP ROW ACCESS POLICY v1 , ADD ROW ACCESS POLICY v2 ON ( v1 ) ;",
            pointer: "/0/alter_object/properties/0/kind",
            expected_value: json!("ADD ROW ACCESS POLICY"),
        },
        ProjectionTestCase {
            description: "a table's ADD after DROP ROW ACCESS POLICY keeps ADD in its kind",
            sql: "ALTER TABLE v1 DROP ROW ACCESS POLICY v1 , ADD ROW ACCESS POLICY v2 ON ( v1 ) ;",
            pointer: "/0/alter_object/properties/0/kind",
            expected_value: json!("ADD ROW ACCESS POLICY"),
        },        ProjectionTestCase {
            description: "WITH before a dynamic Iceberg table's tags is kept in their kind",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 TARGET_LAG = '1 minute' WAREHOUSE = v1 WITH TAG ( v1 = 'm' ) AS SELECT 1 ;",
            pointer: "/0/create_object/properties/2/kind",
            expected_value: json!("WITH TAG"),
        },
        ProjectionTestCase {
            description: "a dynamic Iceberg table's tags without WITH keep the bare kind",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 TARGET_LAG = '1 minute' WAREHOUSE = v1 TAG ( v1 = 'm' ) AS SELECT 1 ;",
            pointer: "/0/create_object/properties/2/kind",
            expected_value: json!("TAG"),
        },
        ProjectionTestCase {
            description: "OR REPLACE on CREATE INDEX is kept",
            sql: "CREATE OR REPLACE INDEX IF NOT EXISTS v1 ON v1 ( v1 ) ;",
            pointer: "/0/create_index/or_replace",
            expected_value: json!(true),
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

#[test]
fn given_a_refresh_boundary_with_time_travel_when_parsing_then_the_clause_is_kept() {
    let test_cases = [
        ProjectionTestCase {
            description: "the call stays the table source",
            sql: "SELECT * FROM DYNAMIC_TABLE_REFRESH_BOUNDARY ( v1 ) AT ( OFFSET => -60 ) ;",
            pointer: "/0/select/from/expressions/0/table/identifier_func/function/name",
            expected_value: json!("DYNAMIC_TABLE_REFRESH_BOUNDARY"),
        },
        ProjectionTestCase {
            description: "AT after the call is its time travel",
            sql: "SELECT * FROM DYNAMIC_TABLE_REFRESH_BOUNDARY ( v1 ) AT ( OFFSET => -60 ) ;",
            pointer: "/0/select/from/expressions/0/table/when/kind",
            expected_value: json!("OFFSET"),
        },
        ProjectionTestCase {
            description: "BEFORE after the call is its time travel",
            sql: "SELECT * FROM DYNAMIC_TABLE_REFRESH_BOUNDARY ( v1 ) BEFORE ( OFFSET => -60 ) ;",
            pointer: "/0/select/from/expressions/0/table/when/this/identifier/name",
            expected_value: json!("BEFORE"),
        },
        ProjectionTestCase {
            description: "CHANGES after the call keeps its information and AT",
            sql: "SELECT * FROM DYNAMIC_TABLE_REFRESH_BOUNDARY ( v1 ) CHANGES ( INFORMATION => DEFAULT ) AT ( OFFSET => -60 ) ;",
            pointer: "/0/select/from/expressions/0/table/changes/at_before/historical_data/kind",
            expected_value: json!("OFFSET"),
        },
        ProjectionTestCase {
            description: "an alias after the clause belongs to the table source",
            sql: "SELECT * FROM DYNAMIC_TABLE_REFRESH_BOUNDARY ( v1 ) AT ( OFFSET => -60 ) AS x ;",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_value: json!("x"),
        },
        ProjectionTestCase {
            description: "the call without a clause stays a call",
            sql: "SELECT * FROM DYNAMIC_TABLE_REFRESH_BOUNDARY ( v1 ) ;",
            pointer: "/0/select/from/expressions/0/function/name",
            expected_value: json!("DYNAMIC_TABLE_REFRESH_BOUNDARY"),
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
