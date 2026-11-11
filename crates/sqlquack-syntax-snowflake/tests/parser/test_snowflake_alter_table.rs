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

#[test]
fn given_snowflake_alter_table_drop_and_rename_of_constraints_when_parsing_then_recorded_verdicts_hold(
) {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "the clustering key dropped as KEY (recorded OK)",
            sql: "ALTER TABLE v1 DROP KEY ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "KEY takes no column list (recorded errno 1003)",
            sql: "ALTER TABLE v1 DROP KEY ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unnamed unique constraint dropped by its columns (recorded OK)",
            sql: "ALTER TABLE v1 DROP UNIQUE ( v1 ) ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "an unnamed foreign key dropped by its columns (recorded OK)",
            sql: "ALTER TABLE v1 DROP FOREIGN KEY ( v1 ) ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a primary key takes no column list (recorded errno 1003)",
            sql: "ALTER TABLE v1 DROP PRIMARY KEY ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a renamed constraint (recorded OK)",
            sql: "ALTER TABLE v1 RENAME CONSTRAINT v1 TO \"v1 Q\" ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a constraint renamed to a reserved word (recorded errno 1003)",
            sql: "ALTER TABLE v1 RENAME CONSTRAINT v1 TO SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a constraint renamed to a string (recorded errno 1003)",
            sql: "ALTER TABLE v1 RENAME CONSTRAINT v1 TO 'v1' ;",
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
fn given_snowflake_table_search_optimization_when_parsing_then_its_targets_are_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "two methods added (recorded OK)",
            sql: "ALTER TABLE IF EXISTS v1 ADD SEARCH OPTIMIZATION ON EQUALITY ( v1 ) , EQUALITY ( v1 ) ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "two columns dropped from an iceberg table (recorded OK)",
            sql: "ALTER ICEBERG TABLE IF EXISTS v1 DROP SEARCH OPTIMIZATION ON v1 , v1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "two expression IDs dropped (recorded OK)",
            sql: "ALTER TABLE IF EXISTS v1 DROP SEARCH OPTIMIZATION ON 1 , 1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "ON with no target (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD SEARCH OPTIMIZATION ON ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved word as the target (recorded errno 1003)",
            sql: "ALTER TABLE v1 DROP SEARCH OPTIMIZATION ON SELECT ;",
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
fn given_snowflake_iceberg_column_actions_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a type change that renames fields (recorded OK)",
            sql: "ALTER ICEBERG TABLE v1 ALTER COLUMN v1 SET DATA TYPE INT RENAME FIELDS ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "SET DATA without TYPE (recorded errno 1003)",
            sql: "ALTER ICEBERG TABLE v1 ALTER COLUMN v1 SET DATA INT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "write defaults on two columns (recorded OK)",
            sql: "ALTER ICEBERG TABLE v1 ALTER v1 SET WRITE DEFAULT 1 , v1 SET WRITE DEFAULT 1 ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a write default without SET (recorded OK)",
            sql: "ALTER ICEBERG TABLE v1 ALTER v1 WRITE DEFAULT 1 ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a write default with no value (recorded errno 1003)",
            sql: "ALTER ICEBERG TABLE v1 ALTER v1 SET WRITE DEFAULT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a dropped write default (recorded OK)",
            sql: "ALTER ICEBERG TABLE v1 ALTER v1 DROP WRITE DEFAULT ;",
            expected_kind: Some("alter_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a dropped column named VERSION_TAG (recorded OK)",
            sql: "ALTER ICEBERG TABLE v1 DROP VERSION_TAG ;",
            expected_kind: Some("alter_table"),
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
fn given_snowflake_reclustering_and_iceberg_refresh_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a manual recluster with a size and a filter (recorded OK)",
            sql: "ALTER TABLE IF EXISTS v1 RECLUSTER MAX_SIZE = 1000000 WHERE TRUE ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "MAX_SIZE with no value (recorded errno 1003)",
            sql: "ALTER TABLE v1 RECLUSTER MAX_SIZE = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "automatic reclustering suspended on an iceberg table (recorded OK)",
            sql: "ALTER ICEBERG TABLE v1 SUSPEND RECLUSTER ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an iceberg refresh from a metadata path (recorded OK)",
            sql: "ALTER ICEBERG TABLE IF EXISTS v1 REFRESH 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an iceberg refresh from a name (recorded errno 1003)",
            sql: "ALTER ICEBERG TABLE v1 REFRESH m1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "REFRESH on a plain table (recorded errno 1003)",
            sql: "ALTER TABLE v1 REFRESH ;",
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
