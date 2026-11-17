//! Each written spelling of one Databricks form keeps its own tree.

use crate::parse::tests::helpers::distinct_trees;
use crate::parse::tests::test_types::DistinctTreesTestCase;

#[test]
fn given_one_form_written_in_different_ways_when_parsing_then_each_tree_keeps_what_was_written() {
    let test_cases = [
        DistinctTreesTestCase {
            description: "a table format written with USING and with STORED AS",
            written: &[
                "CREATE TABLE t3 USING PARQUET",
                "CREATE TABLE t3 STORED AS PARQUET",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a key constraint with and without ENABLE before NOVALIDATE",
            written: &[
                "CREATE TABLE t3 ( c1 INT , PRIMARY KEY ( c1 ) ENABLE NOVALIDATE )",
                "CREATE TABLE t3 ( c1 INT , PRIMARY KEY ( c1 ) NOVALIDATE )",
                "CREATE TABLE t3 ( c1 INT , PRIMARY KEY ( c1 ) ENABLE )",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "DESC and DESCRIBE",
            written: &["DESC SCHEMA s1", "DESCRIBE SCHEMA s1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DESC and DESCRIBE of a connection",
            written: &["DESC CONNECTION c1", "DESCRIBE CONNECTION c1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DESC and DESCRIBE of a function",
            written: &["DESC FUNCTION abs", "DESCRIBE FUNCTION abs"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DESC and DESCRIBE of a table",
            written: &["DESC t1", "DESCRIBE t1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DESCRIBE with and without TABLE",
            written: &["DESCRIBE TABLE t1", "DESCRIBE t1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DESCRIBE with and without QUERY",
            written: &["DESCRIBE QUERY SELECT 1", "DESCRIBE SELECT 1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DESCRIBE SCHEMA, DATABASE and NAMESPACE",
            written: &[
                "DESCRIBE SCHEMA s1",
                "DESCRIBE DATABASE s1",
                "DESCRIBE NAMESPACE s1",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "SHOW TABLES FROM and IN",
            written: &["SHOW TABLES FROM s1", "SHOW TABLES IN s1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "SHOW VIEWS FROM and IN",
            written: &["SHOW VIEWS FROM s1", "SHOW VIEWS IN s1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "SHOW CATALOGS with and without LIKE",
            written: &["SHOW CATALOGS LIKE 'm'", "SHOW CATALOGS 'm'"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "SHOW SCHEMAS, DATABASES and NAMESPACES",
            written: &["SHOW SCHEMAS", "SHOW DATABASES", "SHOW NAMESPACES"],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "SHOW COLUMNS FROM and IN",
            written: &["SHOW COLUMNS FROM t1", "SHOW COLUMNS IN t1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "SHOW COLUMNS with a schema FROM and IN",
            written: &["SHOW COLUMNS IN t1 FROM s1", "SHOW COLUMNS IN t1 IN s1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "SHOW FUNCTIONS keeps a backticked pattern",
            written: &["SHOW FUNCTIONS SELECT", "SHOW FUNCTIONS `SELECT`"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "MINUS and EXCEPT",
            written: &["SELECT 1 MINUS SELECT 2", "SELECT 1 EXCEPT SELECT 2"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DROP COLUMN and DROP COLUMNS",
            written: &[
                "ALTER TABLE t1 DROP COLUMN c1",
                "ALTER TABLE t1 DROP COLUMNS c1",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "ADD COLUMN and ADD COLUMNS",
            written: &[
                "ALTER TABLE t1 ADD COLUMN ( c1 INT )",
                "ALTER TABLE t1 ADD COLUMNS ( c1 INT )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "ALTER and CHANGE of a column",
            written: &[
                "ALTER TABLE t1 ALTER c1 COMMENT 'x'",
                "ALTER TABLE t1 CHANGE c1 COMMENT 'x'",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a backticked field of a dropped column",
            written: &[
                "ALTER TABLE t1 DROP COLUMN ( c1.SELECT )",
                "ALTER TABLE t1 DROP COLUMN ( c1.`SELECT` )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a backticked field of a renamed column",
            written: &[
                "ALTER TABLE t1 RENAME COLUMN c1.SELECT TO c3",
                "ALTER TABLE t1 RENAME COLUMN c1.`SELECT` TO c3",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a backticked field of an added column",
            written: &[
                "ALTER TABLE t1 ADD COLUMN ( c1.SELECT STRING )",
                "ALTER TABLE t1 ADD COLUMN ( c1.`SELECT` STRING )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a backticked field of an altered column",
            written: &[
                "ALTER TABLE t1 ALTER c1.SELECT COMMENT 'x'",
                "ALTER TABLE t1 ALTER c1.`SELECT` COMMENT 'x'",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a backticked field in UPDATE SET",
            written: &[
                "UPDATE t1 SET c1.SELECT = 1",
                "UPDATE t1 SET c1.`SELECT` = 1",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "an added column's collation",
            written: &[
                "ALTER TABLE t1 ADD COLUMN ( c1 STRING )",
                "ALTER TABLE t1 ADD COLUMN ( c1 STRING COLLATE UTF8_LCASE )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a struct field's collation",
            written: &[
                "SELECT CAST ( NULL AS STRUCT < c1 STRING > )",
                "SELECT CAST ( NULL AS STRUCT < c1 STRING COLLATE UTF8_LCASE > )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a backticked struct field",
            written: &[
                "SELECT CAST ( NULL AS STRUCT < SELECT STRING > )",
                "SELECT CAST ( NULL AS STRUCT < `SELECT` STRING > )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a schema's backticked default collation",
            written: &[
                "CREATE SCHEMA s1 DEFAULT COLLATION SELECT",
                "CREATE SCHEMA s1 DEFAULT COLLATION `SELECT`",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a backticked JSON path field",
            written: &["SELECT '{}' : SELECT", "SELECT '{}' : `SELECT`"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "SET of a backticked variable",
            written: &["SET SELECT = 1", "SET `SELECT` = 1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "SET VAR and SET VARIABLE",
            written: &[
                "SET VAR ( v1 ) = ( SELECT 1 )",
                "SET VARIABLE ( v1 ) = ( SELECT 1 )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DECLARE with DEFAULT and =",
            written: &["DECLARE v1 INT DEFAULT 1", "DECLARE v1 INT = 1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "USE CATALOG of a name and of a string",
            written: &["USE CATALOG v1", "USE CATALOG 'v1'"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "COMMENT ON SCHEMA and DATABASE",
            written: &[
                "COMMENT ON SCHEMA s1 IS 'x'",
                "COMMENT ON DATABASE s1 IS 'x'",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "ALTER SCHEMA with and without SET before OWNER",
            written: &[
                "ALTER SCHEMA s1 OWNER TO v1",
                "ALTER SCHEMA s1 SET OWNER TO v1",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "ALTER SCHEMA with and without SET before RETAIN",
            written: &[
                "ALTER SCHEMA s1 RETAIN DROPPED TO 1 HOUR",
                "ALTER SCHEMA s1 SET RETAIN DROPPED TO 1 HOUR",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DROP CONSTRAINT with and without RESTRICT",
            written: &[
                "ALTER TABLE t1 DROP CONSTRAINT n1",
                "ALTER TABLE t1 DROP CONSTRAINT n1 RESTRICT",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "CLONE with and without FOR",
            written: &[
                "CREATE TABLE t3 CLONE t1 FOR SYSTEM_VERSION AS OF 1",
                "CREATE TABLE t3 CLONE t1 SYSTEM_VERSION AS OF 1",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "TEMP and TEMPORARY, with and without AS",
            written: &[
                "REPLACE TEMP TABLE t3 AS SELECT 1",
                "REPLACE TEMPORARY TABLE t3 AS SELECT 1",
                "REPLACE TEMP TABLE t3 SELECT 1",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "a GLOBAL TEMPORARY view",
            written: &[
                "CREATE GLOBAL TEMPORARY VIEW v3 AS SELECT 1",
                "CREATE TEMPORARY VIEW v3 AS SELECT 1",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "BIGINT and LONG",
            written: &[
                "SELECT CAST ( NULL AS BIGINT )",
                "SELECT CAST ( NULL AS LONG )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DECIMAL, DEC and NUMERIC",
            written: &[
                "SELECT CAST ( NULL AS DECIMAL )",
                "SELECT CAST ( NULL AS DEC )",
                "SELECT CAST ( NULL AS NUMERIC )",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "a pipeline from FROM and from TABLE",
            written: &["FROM t1 |> WHERE 1 = 1", "TABLE t1 |> WHERE 1 = 1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a MATCH_RECOGNIZE pattern's quantifiers",
            written: &[
                "SELECT * FROM t1 MATCH_RECOGNIZE ( ORDER BY c1 PATTERN ( a b+ ) DEFINE b AS TRUE )",
                "SELECT * FROM t1 MATCH_RECOGNIZE ( ORDER BY c1 PATTERN ( a b* ) DEFINE b AS TRUE )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "an interval value with and without its + sign",
            written: &[
                "SELECT INTERVAL '1-2' YEAR TO MONTH",
                "SELECT INTERVAL + '1-2' YEAR TO MONTH",
                "SELECT INTERVAL - '1-2' YEAR TO MONTH",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "a VALUES row with and without its parentheses",
            written: &["VALUES ( 1 ), ( 2 )", "VALUES 1, 2", "VALUES 1, ( 2 )"],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "a STREAM source with and without its parentheses",
            written: &["SELECT * FROM STREAM ( t1 )", "SELECT * FROM STREAM t1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "== and =",
            written: &["SELECT 1 == 2", "SELECT 1 = 2"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "!= and <>",
            written: &["SELECT 1 != 2", "SELECT 1 <> 2"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "<=> and IS NOT DISTINCT FROM",
            written: &["SELECT 1 <=> 2", "SELECT 1 IS NOT DISTINCT FROM 2"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "NOT <=> and IS DISTINCT FROM",
            written: &["SELECT NOT 1 <=> 2", "SELECT 1 IS DISTINCT FROM 2"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "REGEXP, RLIKE and the rlike call",
            written: &[
                "SELECT 'a' REGEXP 'b'",
                "SELECT 'a' RLIKE 'b'",
                "SELECT rlike('a', 'b')",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: ">>> and the shiftrightunsigned call",
            written: &["SELECT 4 >>> 1", "SELECT shiftrightunsigned(4, 1)"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "OWNER TO with and without SET",
            written: &[
                "ALTER TABLE t1 OWNER TO u1",
                "ALTER TABLE t1 SET OWNER TO u1",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "SET MANAGED with each mode",
            written: &[
                "ALTER TABLE t1 SET MANAGED",
                "ALTER TABLE t1 SET MANAGED COPY",
                "ALTER TABLE t1 SET MANAGED MOVE",
                "ALTER TABLE t1 SET MANAGED TRUNCATE UNIFORM HISTORY",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "SET EXTERNAL with and without DRY RUN",
            written: &[
                "ALTER TABLE t1 SET EXTERNAL",
                "ALTER TABLE t1 SET EXTERNAL DRY RUN",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "DROP FEATURE with and without TRUNCATE HISTORY",
            written: &[
                "ALTER TABLE t1 DROP FEATURE f1",
                "ALTER TABLE t1 DROP FEATURE f1 TRUNCATE HISTORY",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a column's position after each lead word",
            written: &[
                "ALTER TABLE t1 ALTER c1 FIRST",
                "ALTER TABLE t1 ALTER COLUMN c1 FIRST",
                "ALTER TABLE t1 CHANGE c1 FIRST",
                "ALTER TABLE t1 CHANGE COLUMN c1 FIRST",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "a dropped primary key with each option",
            written: &[
                "ALTER TABLE t1 DROP PRIMARY KEY",
                "ALTER TABLE t1 DROP PRIMARY KEY IF EXISTS",
                "ALTER TABLE t1 DROP PRIMARY KEY CASCADE",
                "ALTER TABLE t1 DROP PRIMARY KEY RESTRICT",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "a row filter with and without columns",
            written: &[
                "ALTER TABLE t1 SET ROW FILTER f1 ON ( )",
                "ALTER TABLE t1 SET ROW FILTER f1 ON ( c1 )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "SET SERDE with and without SERDEPROPERTIES, and SERDEPROPERTIES alone",
            written: &[
                "ALTER TABLE t1 SET SERDE 's1'",
                "ALTER TABLE t1 SET SERDE 's1' WITH SERDEPROPERTIES ( 'k' = 'v' )",
                "ALTER TABLE t1 SET SERDEPROPERTIES ( 'k' = 'v' )",
            ],
            expected_distinct_trees: 3,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            distinct_trees(test_case.written, test_case.description),
            test_case.expected_distinct_trees,
            "{}",
            test_case.description
        );
    }
}
