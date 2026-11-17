//! Databricks forms read as trees whose layout is not reviewed yet stay refused as opaque.

use crate::lint::tests::helpers;
use crate::lint::tests::test_types;

/// The refusal lint gives a statement the native parser keeps as opaque.
const OPAQUE_REFUSAL: &str = "unsupported syntax: native parser retained an opaque SQL node";

#[test]
fn given_a_databricks_form_read_as_a_tree_but_not_reviewed_when_linting_then_it_is_refused_as_opaque()
 {
    let test_cases = [
        test_types::LintRefusalTestCase {
            description: "ADD of resources",
            sql: "ADD FILE f1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "LIST of resources",
            sql: "LIST JARS",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "RESTORE",
            sql: "RESTORE t1 VERSION AS OF 1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "DENY",
            sql: "DENY SELECT ON TABLE t1 TO u1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "CACHE SELECT as documented",
            sql: "CACHE SELECT c1 FROM t1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "REFRESH of a resource path",
            sql: "REFRESH 'p1'",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "DESCRIBE of a column",
            sql: "DESCRIBE TABLE t1 c1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "SHOW of a partition",
            sql: "SHOW PARTITIONS t1 PARTITION ( c1 = 1 )",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a Hive row format",
            sql: "CREATE TABLE t1 ( c1 INT ) ROW FORMAT DELIMITED",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a Hive storage format",
            sql: "CREATE TABLE t1 ( c1 INT ) STORED AS PARQUET",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a constraint's validation words",
            sql: "ALTER TABLE t1 ADD CONSTRAINT k1 CHECK ( c1 > 0 ) ENABLE",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a column mask in CREATE TABLE",
            sql: "CREATE TABLE t1 ( c1 INT MASK m1 )",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a row filter in CREATE TABLE",
            sql: "CREATE TABLE t1 ( c1 INT ) WITH ROW FILTER f1 ON ( c1 )",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a column mask on an added column",
            sql: "ALTER TABLE t1 ADD COLUMN c1 INT MASK m1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a column mask set on a column",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 SET MASK m1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a column mask dropped from a column",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 DROP MASK",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a row filter set on a table",
            sql: "ALTER TABLE t1 SET ROW FILTER f1 ON ( c1 )",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a row filter dropped from a table",
            sql: "ALTER TABLE t1 DROP ROW FILTER",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a table's owner",
            sql: "ALTER TABLE t1 OWNER TO u1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a table set to managed",
            sql: "ALTER TABLE t1 SET MANAGED",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a table set to external",
            sql: "ALTER TABLE t1 SET EXTERNAL",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a table's management unset",
            sql: "ALTER TABLE t1 UNSET MANAGED",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a table's SerDe",
            sql: "ALTER TABLE t1 SET SERDE 's1'",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a table's default collation",
            sql: "ALTER TABLE t1 DEFAULT COLLATION UTF8_BINARY",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "partitions recovered",
            sql: "ALTER TABLE t1 RECOVER PARTITIONS",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "partitioning replaced by clustering",
            sql: "ALTER TABLE t1 REPLACE PARTITIONED BY WITH CLUSTER BY ( c1 )",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "predictive optimization",
            sql: "ALTER TABLE t1 ENABLE PREDICTIVE OPTIMIZATION",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a partition renamed",
            sql: "ALTER TABLE t1 PARTITION ( p1 = 1 ) RENAME TO PARTITION ( p1 = 2 )",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a primary key dropped",
            sql: "ALTER TABLE t1 DROP PRIMARY KEY",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a foreign key dropped",
            sql: "ALTER TABLE t1 DROP FOREIGN KEY ( c1 )",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a table feature dropped",
            sql: "ALTER TABLE t1 DROP FEATURE f1 TRUNCATE HISTORY",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a column's position",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 FIRST",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a column's identity synced",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 SYNC IDENTITY",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a column's tags",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 SET TAGS ( 'k' = 'v' )",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a view's schema binding",
            sql: "CREATE VIEW v1 WITH SCHEMA BINDING AS SELECT 1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a view's default collation",
            sql: "CREATE VIEW v1 DEFAULT COLLATION UTF8_BINARY AS SELECT 1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a view partitioned on columns",
            sql: "CREATE VIEW v1 PARTITIONED ON ( c1 ) AS SELECT 1 AS c1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a view's NOCOPY",
            sql: "CREATE VIEW v1 NOCOPY * AS SELECT 1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a temporary view over a source",
            sql: "CREATE TEMPORARY VIEW v1 USING parquet",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a temporary table dropped",
            sql: "DROP TEMPORARY TABLE t1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a table dropped with FORCE",
            sql: "DROP TABLE t1 FORCE",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a temporary function dropped",
            sql: "DROP TEMPORARY FUNCTION f1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a table undropped by identifier",
            sql: "UNDROP TABLE WITH ID '0'",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "a Hive directory written by INSERT OVERWRITE",
            sql: "INSERT OVERWRITE DIRECTORY '/tmp/x' SELECT 1",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
        test_types::LintRefusalTestCase {
            description: "COPY INTO with its target's columns",
            sql: "COPY INTO t1 ( c1 ) FROM '/tmp/x' FILEFORMAT = CSV",
            dialect: "databricks",
            expected_refusal: OPAQUE_REFUSAL,
        },
    ];
    for test_case in test_cases {
        let refusal: Result<Vec<serde_json::Value>, String> =
            helpers::diagnostics_for_dialect(test_case.sql, test_case.dialect, &["SQLCLEAN001"]);
        assert_eq!(
            refusal.err().as_deref(),
            Some(test_case.expected_refusal),
            "{}",
            test_case.description
        );
    }
}
