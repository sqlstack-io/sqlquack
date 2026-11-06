use crate::helpers::snowflake_kind;
use crate::test_types::SnowflakeVerdictTestCase;

#[test]
fn given_a_snowflake_merge_when_parsing_then_its_actions_need_their_clauses() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a MERGE with no WHEN clause (recorded errno 1003)",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", "", " ;"),
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "UPDATE without SET (recorded errno 1003)",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN MATCHED THEN UPDATE", " ;"),
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET with a string for a column (recorded errno 1003)",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN MATCHED THEN UPDATE SET 'v1' = 1", " ;"),
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "INSERT without VALUES (recorded errno 1003)",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN NOT MATCHED THEN INSERT", " ;"),
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty INSERT column list (recorded errno 1003)",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN NOT MATCHED THEN INSERT ( ) VALUES ( 1 )", " ;"),
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unclosed INSERT column list (recorded errno 1003)",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN NOT MATCHED THEN INSERT ( v1 VALUES ( 1 )", " ;"),
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty VALUES list (recorded errno 1003)",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN NOT MATCHED THEN INSERT VALUES ( )", " ;"),
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unclosed VALUES list (recorded errno 1003)",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN NOT MATCHED THEN INSERT VALUES ( 1", " ;"),
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps MERGE WHEN MATCHED THEN UPDATE SET c1 = t2.c1",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN MATCHED THEN UPDATE SET c1 = t2.c1", " ;"),
            expected_kind: Some("merge"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps MERGE WHEN MATCHED AND t2.c2 > 0 THEN DELETE WHEN NOT MATCHED THE",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN MATCHED AND t2.c2 > 0 THEN DELETE WHEN NOT MATCHED THEN INSERT ( c1 , c2 ) VALUES ( t2.c1 , DEFAULT )", " ;"),
            expected_kind: Some("merge"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps MERGE WHEN NOT MATCHED THEN INSERT VALUES ( 1 )",
            sql: concat!("MERGE INTO v1 USING t2 ON v1.c1 = t2.c1", " WHEN NOT MATCHED THEN INSERT VALUES ( 1 )", " ;"),
            expected_kind: Some("merge"),
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
fn given_a_snowflake_stage_command_when_parsing_then_its_stage_and_options_must_be_complete() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "GET with no stage (recorded errno 1003)",
            sql: "GET file:///tmp/d1/ ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "GET with no file target (recorded errno 1003)",
            sql: "GET @s1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string stage name (recorded errno 1003)",
            sql: "GET @'v1' file:///tmp/d1/ ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string table stage name (recorded errno 1003)",
            sql: "LIST @%'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string namespace part (recorded errno 1003)",
            sql: "REMOVE @'v1'.s1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an option without its value (recorded errno 1003)",
            sql: "LIST @s1 PATTERN = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "LIST with nothing (recorded errno 1003)",
            sql: "LIST ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "REMOVE with nothing (recorded errno 1003)",
            sql: "REMOVE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps GET @~/p1 file:///tmp/d1/ PARALLEL = 1 PATTERN = 'm' ;",
            sql: "GET @~/p1 file:///tmp/d1/ PARALLEL = 1 PATTERN = 'm' ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps GET @s1 'file:///tmp/d 1/' ;",
            sql: "GET @s1 'file:///tmp/d 1/' ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps GET '@%'v1 Q'' file:///tmp/d1/ ;",
            sql: "GET '@%\"v1 Q\"' file:///tmp/d1/ ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps LIST @~ ;",
            sql: "LIST @~ ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps LIST '@'v1 Q'.r1/branches/main' ;",
            sql: "LIST '@\"v1 Q\".r1/branches/main' ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps LIST @v1.s1/p1 PATTERN = 'm' ;",
            sql: "LIST @v1.s1/p1 PATTERN = 'm' ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps REMOVE @%'SELECT' ;",
            sql: "REMOVE @%\"SELECT\" ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps REMOVE @v1.%v1 ;",
            sql: "REMOVE @v1.%v1 ;",
            expected_kind: Some("command"),
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
            description: "keeps ALTER DYNAMIC TABLE v1 SET TARGET_LAG = DOWNSTREAM CONTACT ( STEWARD =",
            sql: "ALTER DYNAMIC TABLE v1 SET TARGET_LAG = DOWNSTREAM CONTACT ( STEWARD = v1 ) ;",
            expected_kind: Some("alter_object"),
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
            description: "keeps ALTER DYNAMIC TABLE v1 MODIFY v1 UNSET TAG a.b , v2 UNSET COMMENT ;",
            sql: "ALTER DYNAMIC TABLE v1 MODIFY v1 UNSET TAG a.b , v2 UNSET COMMENT ;",
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
fn given_a_snowflake_grant_of_a_role_when_parsing_then_its_role_and_grantee_kinds_are_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a role granted to an application (recorded errno 1003)",
            sql: "GRANT ROLE v1 TO APPLICATION v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "no role kind (recorded errno 1003)",
            sql: "GRANT v1 TO ROLE v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "DATABASE without ROLE (recorded errno 1003)",
            sql: "GRANT DATABASE v1 TO USER v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "no grantee kind (recorded errno 1003)",
            sql: "GRANT ROLE v1 TO v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "WITH GRANT without OPTION (recorded errno 1003)",
            sql: "GRANT ROLE v1 TO ROLE v1 WITH GRANT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "GRANT FOR without OPTION (recorded errno 1003)",
            sql: "REVOKE GRANT FOR ROLE v1 FROM ROLE v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a role revoked from a share (recorded errno 1003)",
            sql: "REVOKE ROLE v1 FROM SHARE v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string role (recorded errno 1003)",
            sql: "GRANT APPLICATION ROLE 'v1' TO USER v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps GRANT ROLE v1 TO ROLE v1 WITH GRANT OPTION ;",
            sql: "GRANT ROLE v1 TO ROLE v1 WITH GRANT OPTION ;",
            expected_kind: Some("grant"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps GRANT DATABASE ROLE d1.v1 TO SHARE 'v1 Q' ;",
            sql: "GRANT DATABASE ROLE d1.v1 TO SHARE \"v1 Q\" ;",
            expected_kind: Some("grant"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps GRANT APPLICATION ROLE a1.v1 TO APPLICATION v1 ;",
            sql: "GRANT APPLICATION ROLE a1.v1 TO APPLICATION v1 ;",
            expected_kind: Some("grant"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps REVOKE GRANT OPTION FOR DATABASE ROLE v1 FROM ROLE v1 RESTRICT ;",
            sql: "REVOKE GRANT OPTION FOR DATABASE ROLE v1 FROM ROLE v1 RESTRICT ;",
            expected_kind: Some("revoke"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps REVOKE APPLICATION ROLE v1 FROM USER v1 ;",
            sql: "REVOKE APPLICATION ROLE v1 FROM USER v1 ;",
            expected_kind: Some("revoke"),
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
fn given_a_snowflake_show_of_grants_or_policies_when_parsing_then_its_clauses_are_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "FUTURE without GRANTS (recorded errno 1003)",
            sql: "SHOW FUTURE IN DATABASE v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "FUTURE GRANTS in a bare name (recorded errno 1003)",
            sql: "SHOW FUTURE GRANTS IN v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "GRANTS in a database (recorded errno 1003)",
            sql: "SHOW GRANTS IN DATABASE v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ON with a string kind (recorded errno 1003)",
            sql: "SHOW GRANTS ON 'TABLE' v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ON with no name (recorded errno 1003)",
            sql: "SHOW GRANTS ON TABLE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "OF with no grantee (recorded errno 1003)",
            sql: "SHOW GRANTS OF ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ON with no kind (recorded errno 1003)",
            sql: "SHOW FEATURE POLICIES ON v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ACCOUNT without ON or IN (recorded errno 1003)",
            sql: "SHOW MAINTENANCE POLICIES ACCOUNT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "RULES without NETWORK (recorded errno 1003)",
            sql: "SHOW RULES ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "USER with no kind (recorded errno 1003)",
            sql: "SHOW USER ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "FOR without USER (recorded errno 1003)",
            sql: "SHOW USER KEY PAIRS FOR v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string consumer (recorded errno 1003)",
            sql: "SHOW USER PATS FOR EXTERNAL CONSUMER 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an incomplete routing table (recorded errno 1003)",
            sql: "SHOW EVENT ROUTING TABLE ON ORGANIZATION FOR ALL LISTINGS ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW GRANTS ;",
            sql: "SHOW GRANTS ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW GRANTS ON ACCOUNT LIMIT 1 ;",
            sql: "SHOW GRANTS ON ACCOUNT LIMIT 1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW GRANTS ON EXTERNAL TABLE 'v1 Q' ;",
            sql: "SHOW GRANTS ON EXTERNAL TABLE \"v1 Q\" ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW GRANTS TO SERVICE ROLE v1!'v1 Q' ;",
            sql: "SHOW GRANTS TO SERVICE ROLE v1!\"v1 Q\" ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW GRANTS TO 'SELECT' ROLE v1!v1 ;",
            sql: "SHOW GRANTS TO \"SELECT\" ROLE v1!v1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "LIMIT after IN APPLICATION PACKAGE (recorded errno 1003)",
            sql: "SHOW GRANTS TO SHARE v1 IN APPLICATION PACKAGE v1 LIMIT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW GRANTS OF APPLICATION ROLE v1 LIMIT 1 ;",
            sql: "SHOW GRANTS OF APPLICATION ROLE v1 LIMIT 1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW GRANTS IN DCM PROJECT v1 ;",
            sql: "SHOW GRANTS IN DCM PROJECT v1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW FUTURE GRANTS TO DATABASE ROLE 'v1 Q' ;",
            sql: "SHOW FUTURE GRANTS TO DATABASE ROLE \"v1 Q\" ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW FEATURE POLICIES ON DATABASE 'SELECT' ;",
            sql: "SHOW FEATURE POLICIES ON DATABASE \"SELECT\" ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW MAINTENANCE POLICIES ON ACCOUNT ;",
            sql: "SHOW MAINTENANCE POLICIES ON ACCOUNT ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SHOW MULTI PARTY APPROVAL POLICIES ON ACCOUNT ;",
            sql: "SHOW MULTI PARTY APPROVAL POLICIES ON ACCOUNT ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description:
                "keeps SHOW USER PROGRAMMATIC ACCESS TOKENS FOR EXTERNAL CONSUMER 'v1 Q' ;",
            sql: "SHOW USER PROGRAMMATIC ACCESS TOKENS FOR EXTERNAL CONSUMER \"v1 Q\" ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description:
                "keeps SHOW USER WORKLOAD IDENTITY AUTHENTICATION METHODS FOR USER 'SELECT' ;",
            sql: "SHOW USER WORKLOAD IDENTITY AUTHENTICATION METHODS FOR USER \"SELECT\" ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description:
                "keeps SHOW EVENT ROUTING TABLE ON ORGANIZATION FOR ALL APPLICATION LISTINGS ",
            sql: "SHOW EVENT ROUTING TABLE ON ORGANIZATION FOR ALL APPLICATION LISTINGS ;",
            expected_kind: Some("show"),
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
fn given_a_snowflake_insert_frame_or_execute_when_parsing_then_missing_parts_are_refused() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "WHEN without a condition (recorded errno 1003)",
            sql: "INSERT FIRST WHEN THEN INTO v1 SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "WHEN without THEN (recorded errno 1003)",
            sql: "INSERT FIRST WHEN TRUE INTO v1 SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "INTO without a table (recorded errno 1003)",
            sql: "INSERT ALL INTO SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ELSE without a table (recorded errno 1003)",
            sql: "INSERT FIRST WHEN TRUE THEN INTO v1 ELSE INTO SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a parenthesized value that is not a query (recorded errno 1003)",
            sql: "INSERT INTO v1 ( v1 ) ( 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string frame bound (recorded errno 1003)",
            sql: "SELECT SUM ( 1 ) OVER ( ROWS 'x' PRECEDING ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved experiment name (recorded errno 1003)",
            sql: "EXECUTE EXPERIMENT SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved PURGE alias (recorded errno 1003)",
            sql: "EXECUTE DCM PROJECT v1 PURGE AS SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps INSERT FIRST WHEN a > 1 THEN INTO t1 INTO t2 WHEN a > 2 THEN INTO t3 E",
            sql: "INSERT FIRST WHEN a > 1 THEN INTO t1 INTO t2 WHEN a > 2 THEN INTO t3 ELSE INTO t4 SELECT 1 AS a ;",
            expected_kind: Some("multitable_inserts"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps INSERT ALL INTO t1 INTO t2 ( a ) VALUES ( 1 ) SELECT 1 ;",
            sql: "INSERT ALL INTO t1 INTO t2 ( a ) VALUES ( 1 ) SELECT 1 ;",
            expected_kind: Some("multitable_inserts"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps INSERT INTO t1 ( a ) ( SELECT 1 ) ;",
            sql: "INSERT INTO t1 ( a ) ( SELECT 1 ) ;",
            expected_kind: Some("insert"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SELECT SUM ( a ) OVER ( ORDER BY a ROWS BETWEEN 1 PRECEDING AND CURREN",
            sql: "SELECT SUM ( a ) OVER ( ORDER BY a ROWS BETWEEN 1 PRECEDING AND CURRENT ROW ) FROM t1 ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps EXECUTE EXPERIMENT 'v1 Q' ;",
            sql: "EXECUTE EXPERIMENT \"v1 Q\" ;",
            expected_kind: Some("execute_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps EXECUTE DCM PROJECT v1 PURGE AS a1 ;",
            sql: "EXECUTE DCM PROJECT v1 PURGE AS a1 ;",
            expected_kind: Some("execute_object"),
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
fn given_a_snowflake_alter_table_column_list_when_parsing_then_each_column_and_definition_is_checked(
) {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "UNSET with nothing (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER v1 UNSET ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a collation that is not a string (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD v1 INT COLLATE m1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string identity start (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD v1 INT AUTOINCREMENT START 'x' INCREMENT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved referenced table (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD v1 INT REFERENCES SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved column in the list (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER v1 COMMENT 'm' , SELECT COMMENT 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a table action inside a column list (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER v1 DROP DEFAULT , UNSET COMMENT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "DEFAULT as a column type (recorded errno 1003)",
            sql: "ALTER TABLE v1 ALTER v1 DROP DEFAULT , v2 DEFAULT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an incomplete lifecycle policy kind (recorded errno 1003)",
            sql: "ALTER STORAGE LIFECYCLE v1 RENAME TO v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a lifecycle policy SET with nothing (recorded errno 1003)",
            sql: "ALTER STORAGE LIFECYCLE POLICY v1 SET ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER TABLE IF EXISTS v1 ALTER COLUMN v1 DROP DEFAULT , COLUMN v2 SET ",
            sql: "ALTER TABLE IF EXISTS v1 ALTER COLUMN v1 DROP DEFAULT , COLUMN v2 SET DATA TYPE INT ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER TABLE v1 MODIFY v1 DROP DEFAULT , v2 DROP NOT NULL ;",
            sql: "ALTER TABLE v1 MODIFY v1 DROP DEFAULT , v2 DROP NOT NULL ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER TABLE v1 ALTER v1 UNSET COMMENT , v2 UNSET COMMENT ;",
            sql: "ALTER TABLE v1 ALTER v1 UNSET COMMENT , v2 UNSET COMMENT ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER TABLE v1 ALTER v1 SET DEFAULT s1.NEXTVAL , v2 NOT NULL ;",
            sql: "ALTER TABLE v1 ALTER v1 SET DEFAULT s1.NEXTVAL , v2 NOT NULL ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER TABLE IF EXISTS v1 DROP COLUMN IF EXISTS v1 , v2 ;",
            sql: "ALTER TABLE IF EXISTS v1 DROP COLUMN IF EXISTS v1 , v2 ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER TABLE v1 ADD COLUMN v1 INT CONSTRAINT c1 UNIQUE RELY COLLATE 'm'",
            sql: "ALTER TABLE v1 ADD COLUMN v1 INT CONSTRAINT c1 UNIQUE RELY COLLATE 'm' ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE TABLE t1 ( v1 INT AUTOINCREMENT START -1 INCREMENT 1 , v2 VARCH",
            sql: "CREATE TABLE t1 ( v1 INT AUTOINCREMENT START -1 INCREMENT 1 , v2 VARCHAR COLLATE 'en-ci' ) ;",
            expected_kind: Some("create_table"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER STORAGE LIFECYCLE POLICY IF EXISTS v1 RENAME TO 'v1 Q' ;",
            sql: "ALTER STORAGE LIFECYCLE POLICY IF EXISTS v1 RENAME TO \"v1 Q\" ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER STORAGE LIFECYCLE POLICY v1 SET ARCHIVE_TIER = COOL ;",
            sql: "ALTER STORAGE LIFECYCLE POLICY v1 SET ARCHIVE_TIER = COOL ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER STORAGE LIFECYCLE POLICY v1 UNSET ARCHIVE_FOR_DAYS ;",
            sql: "ALTER STORAGE LIFECYCLE POLICY v1 UNSET ARCHIVE_FOR_DAYS ;",
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
fn given_a_snowflake_create_model_gateway_or_experiment_when_parsing_then_its_source_is_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "FROM without MODEL (recorded errno 1003)",
            sql: "CREATE MODEL v1 FROM v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string source version (recorded errno 1003)",
            sql: "CREATE MODEL v1 FROM MODEL v1 VERSION 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string new version (recorded errno 1003)",
            sql: "CREATE MODEL v1 WITH VERSION 'v1' FROM MODEL v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a gateway with no specification (recorded errno 1003)",
            sql: "CREATE GATEWAY v1 FROM SPECIFICATION ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved experiment name (recorded errno 1003)",
            sql: "CREATE EXPERIMENT SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an INT precision (recorded errno 1003)",
            sql: "CREATE TABLE t1 ( v1 INT ( 1 ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a computed column without a type (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD v1 AS ( 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE OR REPLACE MODEL IF NOT EXISTS v1 WITH VERSION v2 FROM MODEL d1",
            sql: "CREATE OR REPLACE MODEL IF NOT EXISTS v1 WITH VERSION v2 FROM MODEL d1.s1.m1 VERSION last ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE MODEL v1 WITH VERSION 'v1 Q' FROM @s1 ;",
            sql: "CREATE MODEL v1 WITH VERSION \"v1 Q\" FROM @s1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE GATEWAY 'v1 Q' FROM SPECIFICATION $$m: 1$$ ;",
            sql: "CREATE GATEWAY \"v1 Q\" FROM SPECIFICATION $$m: 1$$ ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE EXPERIMENT v1 ;",
            sql: "CREATE EXPERIMENT v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE EXPERIMENT v1 TYPE = 'AI_FUNCTION_EVALUATION' FROM SPECIFICATIO",
            sql: "CREATE EXPERIMENT v1 TYPE = 'AI_FUNCTION_EVALUATION' FROM SPECIFICATION $$v1 $$ ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER TABLE v1 MODIFY DATA METRIC FUNCTION v1 ON ( v1 ) SUSPEND ;",
            sql: "ALTER TABLE v1 MODIFY DATA METRIC FUNCTION v1 ON ( v1 ) SUSPEND ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps ALTER TABLE v1 MODIFY v1 SET MASKING POLICY v1 ;",
            sql: "ALTER TABLE v1 MODIFY v1 SET MASKING POLICY v1 ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE TABLE t1 ( a INT AS ( 1 ) ) ;",
            sql: "CREATE TABLE t1 ( a INT AS ( 1 ) ) ;",
            expected_kind: Some("create_table"),
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
fn given_a_snowflake_stage_or_file_format_when_parsing_then_its_options_need_values() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an option without a value (recorded errno 1003)",
            sql: "CREATE FILE FORMAT v1 TYPE = JSON FILE_EXTENSION = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved name (recorded errno 1003)",
            sql: "CREATE FILE FORMAT SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a column without a type and no query (recorded errno 1003)",
            sql: "CREATE OR REPLACE TEMPORARY TABLE t1 ( c1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a time-travel clause without AT (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 ( TIMESTAMP => TO_TIMESTAMP_LTZ ( '2026-01-01' ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a query as the table body (recorded errno 1003)",
            sql: "CREATE TABLE t1 ( SELECT 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string PIVOT column (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 PIVOT ( SUM ( v1 ) FOR 'v1' IN ( 1 ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved UNPIVOT value column (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 UNPIVOT ( SELECT FOR v1 IN ( v1 ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string name (recorded errno 1003)",
            sql: "CREATE STAGE 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unclosed option list (recorded errno 1003)",
            sql: "CREATE OR ALTER STAGE v1 ENCRYPTION = ( TYPE = 'SNOWFLAKE_FULL' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an extra closing parenthesis (recorded errno 1003)",
            sql: "CREATE OR ALTER FILE FORMAT v1 TYPE = CSV NULL_IF = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a nested option without a value (recorded errno 1003)",
            sql: "DEFINE STAGE d1.s1.v1 URL = 's3://b1/p1/' CREDENTIALS = ( AWS_KEY_ID = AWS_SECRET_KEY = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a nested value without an option (recorded errno 1003)",
            sql: "DEFINE STAGE d1.s1.v1 URL = 's3://b1/p1/' ENCRYPTION = ( = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps DEFINE STAGE d1.s1.v1 FILE_FORMAT = ( TYPE = CSV RECORD_DELIMITER = '~",
            sql: "DEFINE STAGE d1.s1.v1 FILE_FORMAT = ( TYPE = CSV RECORD_DELIMITER = '~~' ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE OR ALTER STAGE v1 FILE_FORMAT = ( TYPE = ORC NULL_IF = ( ) ) ;",
            sql: "CREATE OR ALTER STAGE v1 FILE_FORMAT = ( TYPE = ORC NULL_IF = ( ) ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE STAGE v1 FILE_FORMAT = ( TYPE = JSON TIME_FORMAT = m1 ) ;",
            sql: "CREATE STAGE v1 FILE_FORMAT = ( TYPE = JSON TIME_FORMAT = m1 ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps DEFINE STAGE d1.s1.v1 FILE_FORMAT = ( FORMAT_NAME = m1 ) ;",
            sql: "DEFINE STAGE d1.s1.v1 FILE_FORMAT = ( FORMAT_NAME = m1 ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps DEFINE TEMPORARY FILE FORMAT d1.s1.v1 ;",
            sql: "DEFINE TEMPORARY FILE FORMAT d1.s1.v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE FILE FORMAT v1 TYPE = JSON NULL_IF = ( m1 ) ;",
            sql: "CREATE FILE FORMAT v1 TYPE = JSON NULL_IF = ( m1 ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE STAGE 'SELECT' CLONE v1 ;",
            sql: "CREATE STAGE \"SELECT\" CLONE v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps CREATE TABLE t1 ( c1 , c2 ) AS SELECT 1 , 2 ;",
            sql: "CREATE TABLE t1 ( c1 , c2 ) AS SELECT 1 , 2 ;",
            expected_kind: Some("create_table"),
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
fn given_a_snowflake_task_when_parsing_then_its_properties_follow_the_task_grammar() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "EXECUTE AS USER before the body's AS (recorded parse)",
            sql: "CREATE OR ALTER TASK v1 EXECUTE AS USER \"v1 Q\" AS SELECT 1 ;",
            expected_kind: Some("create_task"),
        },
        SnowflakeVerdictTestCase {
            description:
                "DEFINE TASK with a named predecessor (recorded errno 1622, task not found)",
            sql: "DEFINE TASK d1.s1.v1 AFTER d1.s1.p1 AS SELECT 1 ;",
            expected_kind: Some("create_task"),
        },
        SnowflakeVerdictTestCase {
            description: "a DCM task state word (recorded parse)",
            sql: "DEFINE TASK d1.s1.v1 SUSPENDED AS SELECT 1 ;",
            expected_kind: Some("create_task"),
        },
        SnowflakeVerdictTestCase {
            description: "a task graph has properties and no body (recorded parse)",
            sql: "CREATE TASK GRAPH v1 TASK_DEFAULTS ( USER_TASK_TIMEOUT_MS = 1 ) ;",
            expected_kind: Some("create_task"),
        },
        SnowflakeVerdictTestCase {
            description: "a string predecessor (recorded errno 1003)",
            sql: "CREATE TASK v1 AFTER 'v1' AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved word as a property value (recorded errno 1003)",
            sql: "CREATE TASK v1 ERROR_INTEGRATION = SELECT AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved word as the task name (recorded errno 1003)",
            sql: "CREATE TASK SELECT AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string contact (recorded errno 1003)",
            sql: "CREATE TASK v1 WITH CONTACT ( STEWARD = 'v1' ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a tag without its value (recorded errno 1003)",
            sql: "CREATE TASK v1 WITH TAG ( v1 = ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "WHEN without a condition (recorded errno 1003)",
            sql: "DEFINE TASK d1.s1.v1 WHEN AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "EXECUTE USER without AS (recorded errno 1003)",
            sql: "DEFINE TASK d1.s1.v1 EXECUTE USER v1 AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string task name (recorded errno 1003)",
            sql: "DEFINE TASK 'd1.s1.v1' AS SELECT 1 ;",
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
fn given_a_snowflake_function_or_procedure_when_parsing_then_its_properties_are_typed() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a staged Python handler with IMPORTS and no body (recorded parse)",
            sql: "CREATE FUNCTION v1 ( \"v1Q\" INT ) RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION = '3.10' IMPORTS = ( 'm' ) HANDLER = 'm1.run' ;",
            expected_kind: Some("create_function"),
        },
        SnowflakeVerdictTestCase {
            description: "a Java procedure with PACKAGES and no body (recorded parse)",
            sql: "CREATE PROCEDURE v1 ( v1 INT DEFAULT 1 ) RETURNS INT LANGUAGE JAVA RUNTIME_VERSION = 'm' PACKAGES = ( 'com.snowflake:snowpark:1.14.0' ) HANDLER = 'm' ;",
            expected_kind: Some("create_procedure"),
        },
        SnowflakeVerdictTestCase {
            description: "a service function (recorded parse)",
            sql: "CREATE FUNCTION v1 ( ) RETURNS INT SERVICE = \"v1 Q\" ENDPOINT = e1 AS 'm' ;",
            expected_kind: Some("create_function"),
        },
        SnowflakeVerdictTestCase {
            description: "EXECUTE AS RESTRICTED CALLER (recorded parse)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS INT LANGUAGE JAVASCRIPT EXECUTE AS RESTRICTED CALLER AS 'm' ;",
            expected_kind: Some("create_procedure"),
        },
        SnowflakeVerdictTestCase {
            description: "CALLED ON INPUT without NULL (recorded errno 1003)",
            sql: "CREATE FUNCTION v1 ( ) RETURNS INT LANGUAGE JAVASCRIPT CALLED ON INPUT AS 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "HANDLER = with no string (recorded errno 1003)",
            sql: "CREATE FUNCTION v1 ( ) RETURNS INT LANGUAGE JAVA HANDLER = AS 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "EXECUTE AS with no principal (recorded errno 1003)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS INT LANGUAGE JAVASCRIPT EXECUTE AS AS 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an argument with a type and no name (recorded errno 90201, import 137)",
            sql: "DEFINE PROCEDURE d1.s1.v1 ( INT ) RETURNS INT LANGUAGE JAVASCRIPT AS 'm' ;",
            expected_kind: Some("create_procedure"),
        },
        SnowflakeVerdictTestCase {
            description: "a returned table column with no type (recorded errno 90214, import 137)",
            sql: "DEFINE PROCEDURE d1.s1.v1 ( ) RETURNS TABLE ( v1 ) LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$ ;",
            expected_kind: Some("create_procedure"),
        },
        SnowflakeVerdictTestCase {
            description: "a reserved procedure name (recorded errno 1003)",
            sql: "CREATE PROCEDURE SELECT ( ) RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION = '3.10' HANDLER = 'm' AS 'm' ;",
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
fn given_a_snowflake_table_or_view_when_parsing_then_its_clauses_are_complete() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a view without AS (recorded errno 1003)",
            sql: "DEFINE VIEW d1.s1.v1 SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a view named AS (recorded errno 1003)",
            sql: "DEFINE VIEW AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a row access policy without ON (recorded errno 1003)",
            sql: "CREATE VIEW v1 ROW ACCESS POLICY v1 ( v1 ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string row access policy name (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT ) ROW ACCESS POLICY 'v1' ON ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a trailing comma in the column list (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT , ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "NOT with nothing after it (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT NOT ) ;",
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
fn given_snowflake_security_admin_show_drop_and_describe_when_parsing_then_the_recorded_verdicts_hold(
) {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a policy list is scoped to a user after LIKE (recorded parse)",
            sql: "SHOW AUTHENTICATION POLICIES LIKE 'm' ON USER v1 STARTS WITH 'm' LIMIT 1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "IN after FROM (recorded errno 1003)",
            sql: "SHOW ROW ACCESS POLICIES LIKE 'm' LIMIT 1 FROM 'm' IN ACCOUNT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "IN APPLICATION without a name (recorded parse)",
            sql: "SHOW TABLES IN APPLICATION ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "database roles in a bare name (recorded parse)",
            sql: "SHOW DATABASE ROLES IN v1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "versions in an organization profile (recorded parse)",
            sql: "SHOW VERSIONS IN ORGANIZATION v1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "LIMIT ending the statement names the account (recorded parse)",
            sql: "SHOW BACKUP POLICIES IN ACCOUNT LIMIT ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "LIMIT names the account of SHOW GRANTS ON ACCOUNT (recorded parse)",
            sql: "SHOW GRANTS ON ACCOUNT LIMIT ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "shares in a failover group without its name (recorded parse)",
            sql: "SHOW SHARES IN FAILOVER GROUP ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "listings need the failover group's name (recorded errno 1003)",
            sql: "SHOW LISTINGS IN FAILOVER GROUP ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "roles in a class without its name (recorded parse)",
            sql: "SHOW ROLES IN CLASS ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "service containers need the service's name (recorded errno 1003)",
            sql: "SHOW SERVICE CONTAINERS IN SERVICE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "Openflow data plane integrations take no LIMIT (recorded errno 1003)",
            sql: "SHOW OPENFLOW DATA PLANE INTEGRATIONS LIMIT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "caller grants ON need an object (recorded errno 1003)",
            sql: "SHOW CALLER GRANTS ON ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved word is no package name (recorded errno 1003)",
            sql: "SHOW USER FUNCTIONS IN APPLICATION PACKAGE SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a quoted database role is dropped (recorded parse)",
            sql: "DROP DATABASE ROLE \"v1 Q\" ;",
            expected_kind: Some("drop_object"),
        },
        SnowflakeVerdictTestCase {
            description: "PROFILE names an organization (recorded parse)",
            sql: "DROP ORGANIZATION PROFILE ;",
            expected_kind: Some("drop_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a quoted snapshot policy is dropped (recorded parse)",
            sql: "DROP SNAPSHOT POLICY \"SELECT\" ;",
            expected_kind: Some("drop_object"),
        },
        SnowflakeVerdictTestCase {
            description: "PROFILE names an organization to describe (recorded parse)",
            sql: "DESC ORGANIZATION PROFILE ;",
            expected_kind: Some("describe"),
        },
        SnowflakeVerdictTestCase {
            description: "GRAPH names a task to describe (recorded parse)",
            sql: "DESC TASK GRAPH ;",
            expected_kind: Some("describe"),
        },
        SnowflakeVerdictTestCase {
            description: "an organization is renamed (recorded parse)",
            sql: "ALTER ORGANIZATION v1 RENAME TO v1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "PROFILE names an organization to rename (recorded parse)",
            sql: "ALTER ORGANIZATION PROFILE RENAME TO v1 ;",
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
fn given_snowflake_secret_resource_monitor_and_integration_alters_when_parsing_then_the_recorded_verdicts_hold(
) {
    let test_cases = [
        SnowflakeVerdictTestCase { description: "a secret sets a property (recorded parse)", sql: "ALTER SECRET v1 SET SECRET_STRING = 'm' ;", expected_kind: Some("alter_object") },
        SnowflakeVerdictTestCase { description: "a quoted reserved word names a secret (recorded parse)", sql: "ALTER SECRET \"SELECT\" SET COMMENT = 'm' ;", expected_kind: Some("alter_object") },
        SnowflakeVerdictTestCase { description: "a string is no secret name (recorded errno 1003)", sql: "ALTER SECRET 'v1' SET COMMENT = 'm' ;", expected_kind: None },
        SnowflakeVerdictTestCase { description: "a secret's property needs SET (recorded errno 1003)", sql: "ALTER SECRET v1 COMMENT = 'm' ;", expected_kind: None },
        SnowflakeVerdictTestCase { description: "a resource monitor sets its quota (recorded parse)", sql: "ALTER RESOURCE MONITOR v1 SET CREDIT_QUOTA = 1 ;", expected_kind: Some("alter_object") },
        SnowflakeVerdictTestCase { description: "a resource monitor sets properties and triggers (recorded parse)", sql: "ALTER RESOURCE MONITOR IF EXISTS v1 SET CREDIT_QUOTA = 1 FREQUENCY = MONTHLY TRIGGERS ON 80 PERCENT DO SUSPEND ON 90 PERCENT DO NOTIFY ;", expected_kind: Some("alter_object") },
        SnowflakeVerdictTestCase { description: "a trigger needs PERCENT", sql: "ALTER RESOURCE MONITOR v1 TRIGGERS ON 80 DO SUSPEND ;", expected_kind: None },
        SnowflakeVerdictTestCase { description: "SET needs a property", sql: "ALTER RESOURCE MONITOR v1 SET ;", expected_kind: None },
        SnowflakeVerdictTestCase { description: "an integration refreshes one secret (recorded parse)", sql: "ALTER INTEGRATION v1 REFRESH OAUTH_CLIENT_SECRET_2 ;", expected_kind: Some("alter_object") },
        SnowflakeVerdictTestCase { description: "REFRESH names what it refreshes (recorded errno 1003)", sql: "ALTER INTEGRATION \"v1 Q\" REFRESH ;", expected_kind: None },
        SnowflakeVerdictTestCase { description: "REFRESH names one key (recorded errno 1003)", sql: "ALTER SECURITY INTEGRATION v1 REFRESH SAML2_SNOWFLAKE_PRIVATE_KEY METADATA_URL ;", expected_kind: None },
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
fn given_a_snowflake_copy_when_parsing_then_it_follows_the_recorded_grammar() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "COPY without INTO (recorded errno 1003)",
            sql: "COPY v1 FROM @s1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a load from a plain table name (recorded errno 1003)",
            sql: "COPY INTO v1 FROM m1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string as a stage name (recorded errno 1003)",
            sql: "COPY INTO @'v1' FROM v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unload from a string (recorded errno 1003)",
            sql: "COPY INTO 's3://b1/p1/' FROM 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "PARTITION without BY (recorded errno 1003)",
            sql: "COPY INTO @s1 FROM v1 PARTITION 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved word as an INCLUDE_METADATA key (recorded errno 1003)",
            sql: "COPY INTO v1 FROM @s1 INCLUDE_METADATA = ( SELECT = METADATA$FILENAME ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty file format value (recorded errno 1003)",
            sql: "COPY INTO v1 FROM @s1 FILE_FORMAT = ( TYPE = JSON DATE_FORMAT = ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a credential without a key (recorded errno 1003)",
            sql: "COPY INTO v1 FROM 's3://b1/p1/' CREDENTIALS = ( = 'm' AWS_SECRET_KEY = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string key after the first INCLUDE_METADATA item (recorded errno 1003)",
            sql: "COPY INTO v1 FROM @s1 INCLUDE_METADATA = ( v1 = METADATA$FILENAME , 'v1' = METADATA$FILENAME ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty select list in COPY FILES (recorded errno 1003)",
            sql: "COPY FILES INTO @s1 FROM ( SELECT FROM v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty top-level option value (recorded errno 1003)",
            sql: "COPY INTO v1 FROM 's3://b1/p1/' ENCRYPTION = ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a load from the table's own stage names no source",
            sql: "COPY INTO v1 FILE_FORMAT = ( TYPE = CSV ) ;",
            expected_kind: Some("copy"),
        },
        SnowflakeVerdictTestCase {
            description: "a stage after the table still needs FROM",
            sql: "COPY INTO v1 @s1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unload still needs FROM",
            sql: "COPY INTO 's3://b1/p1/' v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "the connector's load from a bind placeholder",
            sql: "COPY INTO \"T\" FROM ? ;",
            expected_kind: Some("copy"),
        },
        SnowflakeVerdictTestCase {
            description: "the connector's IDENTIFIER(?) target with a column list",
            sql: "COPY INTO identifier(?) ( \"A\" ) FROM ( SELECT $1:\"A\" FROM '@\"STG\"' ) ON_ERROR = ? PURGE = TRUE ;",
            expected_kind: Some("copy"),
        },
        SnowflakeVerdictTestCase {
            description: "a typed load stays a COPY tree",
            sql: "COPY INTO v1 FROM @s1 FILE_FORMAT = ( TYPE = CSV ) ;",
            expected_kind: Some("copy"),
        },
        SnowflakeVerdictTestCase {
            description: "COPY FILES from a query parses (recorded errno 90105)",
            sql: "COPY FILES INTO @s1 FROM ( SELECT v1 , 'v1' FROM v1 ) ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "COPY FILES into the bare stage @ parses (recorded errno 1012)",
            sql: "COPY FILES INTO @ FROM @s1 ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "an unload with PARTITION BY parses (recorded errno 90105)",
            sql: "COPY INTO @s1 FROM v1 PARTITION BY 1 ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "a string key as the only INCLUDE_METADATA item parses (recorded errno 1757)",
            sql: "COPY INTO v1 FROM @s1 INCLUDE_METADATA = ( 'v1' = METADATA$FILENAME ) ;",
            expected_kind: Some("command"),
        },
        SnowflakeVerdictTestCase {
            description: "a $ column named by a reserved word parses (recorded errno 2211)",
            sql: "COPY INTO v1 FROM ( SELECT $SELECT FROM @s1 ) ;",
            expected_kind: Some("command"),
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
fn given_a_snowflake_after_match_skip_to_when_parsing_then_a_bare_symbol_is_accepted() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "NEXT without ROW is the symbol of TO <symbol> (recorded errno 2347)",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( AFTER MATCH SKIP TO NEXT PATTERN ( a ) DEFINE a AS 1 ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "TO NEXT ROW stays the next-row form",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( AFTER MATCH SKIP TO NEXT ROW PATTERN ( a ) DEFINE a AS 1 ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "TO needs a symbol",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( AFTER MATCH SKIP TO PATTERN ( a ) DEFINE a AS 1 ) ;",
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
fn given_a_snowflake_from_clause_when_parsing_then_recorded_query_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an unknown call in FROM (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 ( 10 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ERROR_TABLE of a string (recorded errno 1003)",
            sql: "SELECT 1 FROM ERROR_TABLE ( 'v1' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty CHANGES is accepted (recorded OK)",
            sql: "SELECT 1 FROM v1 CHANGES ( ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "JOIN without FROM (recorded errno 1003)",
            sql: "SELECT 1 t1 JOIN t2 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "MATCH_CONDITION on an inner join (recorded errno 1003)",
            sql: "SELECT * FROM t1 JOIN t2 MATCH_CONDITION ( t1.c1 >= t2.c1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a negative sample size (recorded errno 1003)",
            sql: "SELECT * FROM t1 SAMPLE ( -1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a negative sample seed (recorded errno 1003)",
            sql: "SELECT * FROM t1 SAMPLE SYSTEM ( 10 ) SEED ( -1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps an unsigned sample seed",
            sql: "SELECT * FROM t1 SAMPLE SYSTEM ( 10 ) SEED ( 1 ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "a trailing comma in a VALUES row (recorded errno 1003)",
            sql: "SELECT 1 FROM ( VALUES ( 1 , ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved column alias (recorded errno 1003)",
            sql: "SELECT 1 FROM ( VALUES ( 1 ) ) v1 ( SELECT ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps a quoted reserved column alias",
            sql: "SELECT 1 FROM ( VALUES ( 1 ) ) v1 ( \"SELECT\" ) ;",
            expected_kind: Some("select"),
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
fn given_snowflake_match_recognize_and_semantic_view_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an empty PARTITION BY takes PATTERN as its item (recorded errno 1003)",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( PARTITION BY PATTERN ( a ) DEFINE a AS 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty PATTERN (recorded errno 1003)",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( PATTERN ( ) DEFINE a AS 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unclosed exclusion in PATTERN (recorded errno 1003)",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( ORDER BY c1 PATTERN ( {- a b ) DEFINE a AS TRUE , b AS TRUE ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a measure without an alias (recorded errno 1003)",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( MEASURES 1 PATTERN ( a ) DEFINE a AS 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a measure alias without AS (recorded OK)",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( MEASURES 1 v1 PATTERN ( a ) DEFINE a AS 1 ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "SHOW EMPTY MATCHES (recorded OK)",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( ALL ROWS PER MATCH SHOW EMPTY MATCHES PATTERN ( a ) DEFINE a AS 1 ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "SEMANTIC_VIEW with no clause (recorded errno 1003)",
            sql: "SELECT * FROM SEMANTIC_VIEW ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SEMANTIC_VIEW of a string (recorded errno 1003)",
            sql: "SELECT * FROM SEMANTIC_VIEW ( 'v1' METRICS 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a SEMANTIC_VIEW alias without AS (recorded OK)",
            sql: "SELECT * FROM SEMANTIC_VIEW ( v1 METRICS 1 DIMENSIONS 1 v1 ) ;",
            expected_kind: Some("select"),
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
