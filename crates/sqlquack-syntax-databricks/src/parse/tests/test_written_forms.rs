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
            description: "the NOT of SET NOT NULL, DROP NOT NULL and WHEN NOT MATCHED written as NOT and as !",
            written: &[
                "ALTER TABLE t1 ALTER c1 SET NOT NULL",
                "ALTER TABLE t1 ALTER c1 SET ! NULL",
                "ALTER TABLE t1 CHANGE c1 DROP NOT NULL",
                "ALTER TABLE t1 CHANGE c1 DROP ! NULL",
                "MERGE INTO t1 USING t2 ON t1.c1 = t2.c1 WHEN NOT MATCHED BY SOURCE THEN DELETE",
                "MERGE INTO t1 USING t2 ON t1.c1 = t2.c1 WHEN ! MATCHED BY SOURCE THEN DELETE",
            ],
            expected_distinct_trees: 6,
        },
        DistinctTreesTestCase {
            description: "a FOREIGN KEY with and without the constraint options DEFERRABLE and INITIALLY DEFERRED",
            written: &[
                "CREATE TABLE t3 ( c1 INT , CONSTRAINT k1 FOREIGN KEY ( c1 ) REFERENCES t2 )",
                "CREATE TABLE t3 ( c1 INT , CONSTRAINT k1 FOREIGN KEY ( c1 ) REFERENCES t2 DEFERRABLE )",
                "CREATE TABLE t3 ( c1 INT , CONSTRAINT k1 FOREIGN KEY ( c1 ) REFERENCES t2 INITIALLY DEFERRED )",
                "CREATE TABLE t3 ( c1 INT , CONSTRAINT k1 FOREIGN KEY ( c1 ) REFERENCES t2 DEFERRABLE INITIALLY DEFERRED )",
                "CREATE TABLE t3 ( c1 INT , CONSTRAINT k1 FOREIGN KEY ( c1 ) REFERENCES t2 INITIALLY DEFERRED DEFERRABLE )",
            ],
            expected_distinct_trees: 5,
        },
        DistinctTreesTestCase {
            description: "a query written FROM first and SELECT first, with and without a pipe",
            written: &[
                "FROM t1 |> WHERE 1 = 1",
                "SELECT * FROM t1 |> WHERE 1 = 1",
                "FROM t1 SELECT c1",
                "SELECT c1 FROM t1",
                "FROM t1 SELECT *",
                "FROM t1 |> SELECT *",
            ],
            expected_distinct_trees: 6,
        },
        DistinctTreesTestCase {
            description: "a PIVOT and UNPIVOT pipe after FROM t1, TABLE t1 and SELECT * FROM t1",
            written: &[
                "FROM t1 |> PIVOT ( sum ( c1 ) FOR c2 IN ( 1 , 2 ) )",
                "TABLE t1 |> PIVOT ( sum ( c1 ) FOR c2 IN ( 1 , 2 ) )",
                "SELECT * FROM t1 |> PIVOT ( sum ( c1 ) FOR c2 IN ( 1 , 2 ) )",
                "FROM t1 |> UNPIVOT ( v FOR k IN ( c1 , c2 ) )",
                "TABLE t1 |> UNPIVOT ( v FOR k IN ( c1 , c2 ) )",
                "SELECT * FROM t1 |> UNPIVOT ( v FOR k IN ( c1 , c2 ) )",
            ],
            expected_distinct_trees: 6,
        },
        DistinctTreesTestCase {
            description: "a table tag's value written after = and without it",
            written: &[
                "ALTER TABLE t1 SET TAGS ( 'k' = 'v' )",
                "ALTER TABLE t1 SET TAGS ( 'k' 'v' )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a resource written unquoted, backquoted and as a string",
            written: &["ADD FILE v1", "ADD FILE `v1`", "ADD FILE 'v1'"],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "a returned table's columns with different spacing and comments",
            written: &[
                "CREATE FUNCTION f1 ( ) RETURNS TABLE ( c1 INT , c2 STRING ) RETURN SELECT 1 , 'a'",
                "CREATE FUNCTION f1 ( ) RETURNS TABLE(c1 INT, c2 STRING) RETURN SELECT 1 , 'a'",
                "CREATE FUNCTION f1 ( ) RETURNS TABLE (\n  c1 INT, -- x\n  c2 STRING\n) RETURN SELECT 1 , 'a'",
                "CREATE FUNCTION f1 ( ) RETURNS TABLE ( c1 BIGINT , c2 STRING ) RETURN SELECT 1 , 'a'",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a resource written as a raw string and as a string",
            written: &[
                "ADD FILE r'v1'",
                "ADD FILE 'v1'",
                "ADD JAR r'v1'",
                "ADD JAR 'v1'",
                "LIST FILE r'v1'",
                "LIST FILE 'v1'",
            ],
            expected_distinct_trees: 6,
        },
        DistinctTreesTestCase {
            description: "a path key written as a raw string and as a string",
            written: &[
                "SELECT c1 : [ r'k' ] FROM t1",
                "SELECT c1 : [ 'k' ] FROM t1",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a listed resource written backquoted and as a string holding the backquotes",
            written: &["LIST FILE `v1 Q`", "LIST FILE '`v1 Q`'"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a shown property key written as a name, backquoted and as a string",
            written: &[
                "SHOW TBLPROPERTIES t1 ( k1 )",
                "SHOW TBLPROPERTIES t1 ( `k1` )",
                "SHOW TBLPROPERTIES t1 ( 'k1' )",
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
            description: "Hive GROUPING SETS after the item list and a comma before the sets",
            written: &[
                "SELECT c1 FROM t1 GROUP BY c1 GROUPING SETS ( c1 )",
                "SELECT c1 FROM t1 GROUP BY c1 , GROUPING SETS ( c1 )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "TIMESTAMP and its synonym TIMESTAMP_LTZ",
            written: &[
                "SELECT CAST ( NULL AS TIMESTAMP )",
                "SELECT CAST ( NULL AS TIMESTAMP_LTZ )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "USE CATALOG and SET CATALOG",
            written: &["USE CATALOG v1", "SET CATALOG v1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a pipeline from FROM and from TABLE",
            written: &["FROM t1 |> WHERE 1 = 1", "TABLE t1 |> WHERE 1 = 1"],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "LIMIT piped and written after a set operation",
            written: &[
                "SELECT 1 UNION SELECT 2 |> LIMIT 1",
                "SELECT 1 UNION SELECT 2 LIMIT 1",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "ORDER BY piped and written after a set operation",
            written: &[
                "SELECT 1 UNION SELECT 2 |> ORDER BY 1",
                "SELECT 1 UNION SELECT 2 ORDER BY 1",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "OFFSET, CLUSTER BY, DISTRIBUTE BY and SORT BY piped after a set operation",
            written: &[
                "SELECT 1 UNION SELECT 2 |> OFFSET 1",
                "SELECT 1 UNION SELECT 2 OFFSET 1",
                "SELECT 1 UNION SELECT 2 |> CLUSTER BY 1",
                "SELECT 1 UNION SELECT 2 CLUSTER BY 1",
                "SELECT 1 UNION SELECT 2 |> DISTRIBUTE BY 1",
                "SELECT 1 UNION SELECT 2 DISTRIBUTE BY 1",
                "SELECT 1 UNION SELECT 2 |> SORT BY 1",
                "SELECT 1 UNION SELECT 2 SORT BY 1",
            ],
            expected_distinct_trees: 8,
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
            description: "CACHE TABLE OPTIONS with and without =",
            written: &[
                "CACHE TABLE t1 OPTIONS ( 'storageLevel' = 'DISK_ONLY' )",
                "CACHE TABLE t1 OPTIONS ( 'storageLevel' 'DISK_ONLY' )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "RETAIN DROPPED in the singular and plural unit",
            written: &[
                "ALTER SCHEMA s1 RETAIN DROPPED TO 1 DAY",
                "ALTER SCHEMA s1 RETAIN DROPPED TO 1 DAYS",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "INSERT INTO with and without TABLE, and with WITH SCHEMA EVOLUTION",
            written: &[
                "INSERT INTO t1 SELECT 1",
                "INSERT INTO TABLE t1 SELECT 1",
                "INSERT WITH SCHEMA EVOLUTION INTO t1 SELECT 1",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "an insert's query in no, one and two pairs of parentheses",
            written: &[
                "INSERT INTO t1 SELECT 1",
                "INSERT INTO t1 ( SELECT 1 )",
                "INSERT INTO t1 ( ( SELECT 1 ) )",
                "INSERT INTO t1 VALUES ( 1 )",
                "INSERT INTO t1 ( VALUES ( 1 ) )",
                "INSERT OVERWRITE t1 SELECT 1",
                "INSERT OVERWRITE t1 ( SELECT 1 )",
                "INSERT INTO t1 ( c1 ) SELECT 1",
                "INSERT INTO t1 ( c1 ) ( SELECT 1 )",
                "INSERT INTO t1 ( c1 ) ( ( SELECT 1 ) )",
                "INSERT INTO t1 ( c1 ) VALUES ( 1 )",
                "INSERT INTO t1 ( c1 ) ( VALUES ( 1 ) )",
            ],
            expected_distinct_trees: 12,
        },
        DistinctTreesTestCase {
            description: "a pipe operator after a query in no, one and two pairs of parentheses",
            written: &[
                "SELECT 1 |> WHERE 1 = 1",
                "( SELECT 1 ) |> WHERE 1 = 1",
                "( ( SELECT 1 ) ) |> WHERE 1 = 1",
                "SELECT * FROM ( SELECT 1 ) |> WHERE 1 = 1",
                "SELECT * FROM ( ( SELECT 1 ) ) |> WHERE 1 = 1",
                "( SELECT 1 ) |> TABLESAMPLE ( 1 ROWS )",
                "SELECT * FROM ( ( SELECT 1 ) ) |> TABLESAMPLE ( 1 ROWS )",
                "SELECT * FROM ( SELECT 1 ) WHERE 1 = 1 |> WHERE 2 = 2",
                "SELECT 1 |> WHERE 1 = 1 |> WHERE 2 = 2",
                "SELECT * FROM ( SELECT 1 ) a1 |> WHERE 1 = 1",
            ],
            expected_distinct_trees: 10,
        },
        DistinctTreesTestCase {
            description: "REPLACE ON's query with and without parentheses",
            written: &[
                "INSERT INTO t1 REPLACE ON 1 = 1 SELECT 1",
                "INSERT INTO t1 REPLACE ON 1 = 1 ( SELECT 1 )",
            ],
            expected_distinct_trees: 2,
        },
        DistinctTreesTestCase {
            description: "a directory's OPTIONS with and without =, and its path written or as an option",
            written: &[
                "INSERT OVERWRITE DIRECTORY USING PARQUET OPTIONS ( 'path' = '/tmp/x' ) SELECT 1",
                "INSERT OVERWRITE DIRECTORY USING PARQUET OPTIONS ( 'path' '/tmp/x' ) SELECT 1",
                "INSERT OVERWRITE DIRECTORY '/tmp/x' USING PARQUET SELECT 1",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "DROP CONSTRAINT with neither, RESTRICT or CASCADE",
            written: &[
                "ALTER TABLE t1 DROP CONSTRAINT n1",
                "ALTER TABLE t1 DROP CONSTRAINT n1 RESTRICT",
                "ALTER TABLE t1 DROP CONSTRAINT n1 CASCADE",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "DROP PARTITION with or without PURGE, and UNSET TBLPROPERTIES with or without IF EXISTS",
            written: &[
                "ALTER TABLE t1 DROP PARTITION ( c1 = 1 )",
                "ALTER TABLE t1 DROP PARTITION ( c1 = 1 ) PURGE",
                "ALTER TABLE t1 UNSET TBLPROPERTIES ( 'k' )",
                "ALTER TABLE t1 UNSET TBLPROPERTIES IF EXISTS ( 'k' )",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "an unset table property key written as a name, backquoted and as a string",
            written: &[
                "ALTER TABLE t1 UNSET TBLPROPERTIES ( k )",
                "ALTER TABLE t1 UNSET TBLPROPERTIES ( `k` )",
                "ALTER TABLE t1 UNSET TBLPROPERTIES ( 'k' )",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "an unset view property key as a name or a string, with or without IF EXISTS (recorded accept alter-view.k897fc9e5f983)",
            written: &[
                "ALTER VIEW v1 UNSET TBLPROPERTIES ( 'k' )",
                "ALTER VIEW v1 UNSET TBLPROPERTIES ( k )",
                "ALTER VIEW v1 UNSET TBLPROPERTIES IF EXISTS ( 'k' )",
                "ALTER VIEW v1 UNSET TBLPROPERTIES IF EXISTS ( k )",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "a view property's key and value as written: string, name, number, boolean (recorded accept alter-view.k83dd4f39ba83)",
            written: &[
                "ALTER VIEW v1 SET TBLPROPERTIES ( 'k' = 'v' )",
                "ALTER VIEW v1 SET TBLPROPERTIES ( 'k' = 1 )",
                "ALTER VIEW v1 SET TBLPROPERTIES ( 'k' = '1' )",
                "ALTER VIEW v1 SET TBLPROPERTIES ( k = 1 )",
                "ALTER VIEW v1 SET TBLPROPERTIES ( 'k' = true )",
                "ALTER VIEW v1 SET TBLPROPERTIES ( 'k' = 'true' )",
            ],
            expected_distinct_trees: 6,
        },
        DistinctTreesTestCase {
            description: "a view's relationship constraint: named or not, cardinality, ON or USING, aliased or not",
            written: &[
                "ALTER VIEW v1 ADD CONSTRAINT k1 MANY TO ONE RELATIONSHIP TO v2 ON 1 = 1",
                "ALTER VIEW v1 ADD MANY TO ONE RELATIONSHIP TO v2 ON 1 = 1",
                "ALTER VIEW v1 ADD CONSTRAINT k1 ONE TO MANY RELATIONSHIP TO v2 ON 1 = 1",
                "ALTER VIEW v1 ADD CONSTRAINT k1 MANY TO ONE RELATIONSHIP TO v2 USING ( c1 )",
                "ALTER VIEW v1 ADD CONSTRAINT k1 MANY TO ONE RELATIONSHIP TO v2 AS a1 ON 1 = 1",
            ],
            expected_distinct_trees: 5,
        },
        DistinctTreesTestCase {
            description: "CACHE SELECT's FROM as a clause, as an alias, and as a column",
            written: &[
                "CACHE SELECT c1 FROM t1",
                "CACHE SELECT c1 FROM",
                "CACHE SELECT FROM t1",
                "CACHE SELECT c1 FROM t1 WHERE",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "NEAREST BY: APPROX or EXACT, with or without a count, DISTANCE or SIMILARITY",
            written: &[
                "SELECT * FROM t1 JOIN t2 APPROX NEAREST BY DISTANCE f1 ( 1 )",
                "SELECT * FROM t1 JOIN t2 EXACT NEAREST BY DISTANCE f1 ( 1 )",
                "SELECT * FROM t1 JOIN t2 APPROX NEAREST 1 BY DISTANCE f1 ( 1 )",
                "SELECT * FROM t1 JOIN t2 APPROX NEAREST BY SIMILARITY f1 ( 1 )",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "a WATERMARK expression bare, aliased, aliased with AS, and with AS ( aliases )",
            written: &[
                "SELECT * FROM t1 WATERMARK c1 DELAY OF INTERVAL 1 MINUTE",
                "SELECT * FROM t1 WATERMARK c1 a2 DELAY OF INTERVAL 1 MINUTE",
                "SELECT * FROM t1 WATERMARK c1 AS a2 DELAY OF INTERVAL 1 MINUTE",
                "SELECT * FROM t1 WATERMARK c1 AS ( a2 ) DELAY OF INTERVAL 1 MINUTE",
            ],
            expected_distinct_trees: 4,
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
        DistinctTreesTestCase {
            description: "a table in one, two and three pairs of parentheses",
            written: &[
                "SELECT * FROM t1",
                "SELECT * FROM ( t1 )",
                "SELECT * FROM ( ( t1 ) )",
                "SELECT * FROM ( ( ( t1 ) ) )",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "a table function, a join and an inline table, bare and in parentheses",
            written: &[
                "SELECT * FROM range ( 1 ) a1",
                "SELECT * FROM ( range ( 1 ) a1 )",
                "SELECT * FROM ( range ( 1 ) ) a1",
                "SELECT * FROM ( t1 JOIN t2 )",
                "SELECT * FROM ( ( t1 JOIN t2 ) )",
                "SELECT * FROM VALUES ( 1 ) a1",
                "SELECT * FROM ( VALUES ( 1 ) ) a1",
                "SELECT * FROM ( ( VALUES ( 1 ) ) ) a1",
            ],
            expected_distinct_trees: 8,
        },
        DistinctTreesTestCase {
            description: "a query in one and two pairs of parentheses",
            written: &[
                "SELECT * FROM ( SELECT 1 )",
                "SELECT * FROM ( ( SELECT 1 ) )",
                "SELECT * FROM abs ( TABLE ( SELECT 1 ) )",
                "SELECT * FROM abs ( TABLE ( ( SELECT 1 ) ) )",
                "VACUUM t1 USING INVENTORY ( SELECT 1 )",
                "VACUUM t1 USING INVENTORY ( ( SELECT 1 ) )",
                "SELECT 1 UNION SELECT 2",
                "( SELECT 1 ) UNION SELECT 2",
                "CREATE TABLE t1 AS ( SELECT 1 )",
                "CREATE TABLE t1 AS ( ( SELECT 1 ) )",
                "CREATE VIEW v1 AS ( SELECT 1 )",
                "CREATE VIEW v1 AS ( ( SELECT 1 ) )",
            ],
            expected_distinct_trees: 12,
        },
        DistinctTreesTestCase {
            description: "a parenthesized query with a clause after it and a star over it",
            written: &[
                "( SELECT 1 ) ORDER BY 1",
                "SELECT * FROM ( ( SELECT 1 ) ) ORDER BY 1",
                "SELECT 1 ORDER BY 1",
                "( SELECT 1 ) LIMIT 1",
                "SELECT * FROM ( ( SELECT 1 ) ) LIMIT 1",
                "( ( SELECT 1 ) ) LIMIT 1",
                "( SELECT 1 LIMIT 1 )",
            ],
            expected_distinct_trees: 7,
        },
        DistinctTreesTestCase {
            description: "CREATE TABLE AS a parenthesized query with LIMIT, and a star over it",
            written: &[
                "CREATE TABLE t1 AS ( SELECT 1 ) LIMIT 1",
                "CREATE TABLE t1 AS SELECT * FROM ( ( SELECT 1 ) ) LIMIT 1",
                "CREATE TABLE t1 AS SELECT 1 LIMIT 1",
                "CREATE TABLE t1 AS ( SELECT 1 LIMIT 1 )",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "WITH before a parenthesized query, inside it, and over a star",
            written: &[
                "WITH c AS ( SELECT 1 ) ( SELECT * FROM c )",
                "WITH c AS ( SELECT 1 ) SELECT * FROM ( ( SELECT * FROM c ) )",
                "WITH c AS ( SELECT 1 ) SELECT * FROM c",
                "( WITH c AS ( SELECT 1 ) SELECT * FROM c )",
                "WITH c AS ( SELECT 1 ) ( ( SELECT * FROM c ) )",
                "WITH c AS ( SELECT 1 ) ( SELECT * FROM c ) ORDER BY 1",
                "( WITH c AS ( SELECT 1 ) SELECT * FROM c ) ORDER BY 1",
                "WITH c AS ( SELECT 1 ) ( SELECT * FROM c LIMIT 1 ) ORDER BY 1",
                "( WITH c AS ( SELECT 1 ) SELECT * FROM c LIMIT 1 ) ORDER BY 1",
                "WITH c AS ( SELECT 1 ) ( SELECT * FROM c ) |> WHERE 1 = 1",
                "( WITH c AS ( SELECT 1 ) SELECT * FROM c ) |> WHERE 1 = 1",
            ],
            expected_distinct_trees: 11,
        },
        DistinctTreesTestCase {
            description: "clauses after a parenthesized query apart from the same clauses inside it",
            written: &[
                "( SELECT 1 ) ORDER BY 1 LIMIT 1",
                "( SELECT 1 ORDER BY 1 ) LIMIT 1",
                "( ( SELECT 1 ) ) ORDER BY 1 LIMIT 1",
                "( ( SELECT 1 ORDER BY 1 ) ) LIMIT 1",
                "( ( ( SELECT 1 ) ) ) ORDER BY 1 LIMIT 1",
                "( ( ( SELECT 1 ORDER BY 1 ) ) ) LIMIT 1",
                "CREATE TABLE t1 AS ( SELECT 1 ) ORDER BY 1 LIMIT 1",
                "CREATE TABLE t1 AS ( SELECT 1 ORDER BY 1 ) LIMIT 1",
                "SELECT 1 WHERE 1 IN ( ( SELECT 1 ) ORDER BY 1 LIMIT 1 )",
                "SELECT 1 WHERE 1 IN ( ( SELECT 1 ORDER BY 1 ) LIMIT 1 )",
                "SELECT ( ( SELECT 1 ) ORDER BY 1 LIMIT 1 )",
                "SELECT ( ( SELECT 1 ORDER BY 1 ) LIMIT 1 )",
                "( WITH c AS ( SELECT 1 ) SELECT 1 ) ORDER BY 1",
                "WITH c AS ( SELECT 1 ) ( SELECT 1 ORDER BY 1 )",
            ],
            expected_distinct_trees: 14,
        },
        DistinctTreesTestCase {
            description: "WITH before a parenthesized query with a clause after it apart from one more pair",
            written: &[
                "WITH c AS ( SELECT 1 ) ( SELECT 1 ) ORDER BY 1",
                "WITH c AS ( SELECT 1 ) ( ( SELECT 1 ORDER BY 1 ) )",
                "WITH c AS ( SELECT 1 ) ( SELECT 1 ) LIMIT 1",
                "WITH c AS ( SELECT 1 ) ( ( SELECT 1 LIMIT 1 ) )",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "TIMESTAMP bare, WITH LOCAL TIME ZONE and WITHOUT TIME ZONE",
            written: &[
                "SELECT CAST ( v1 AS TIMESTAMP )",
                "SELECT CAST ( v1 AS TIMESTAMP WITH LOCAL TIME ZONE )",
                "SELECT CAST ( v1 AS TIMESTAMP WITHOUT TIME ZONE )",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "a COPY credential value written as a string, a number and a boolean",
            written: &[
                "COPY INTO t1 FROM 's3://b/p' WITH ( CREDENTIAL ( k = '1' ) ) FILEFORMAT = CSV",
                "COPY INTO t1 FROM 's3://b/p' WITH ( CREDENTIAL ( k = 1 ) ) FILEFORMAT = CSV",
                "COPY INTO t1 FROM 's3://b/p' WITH ( CREDENTIAL ( k = 'true' ) ) FILEFORMAT = CSV",
                "COPY INTO t1 FROM 's3://b/p' WITH ( CREDENTIAL ( k = true ) ) FILEFORMAT = CSV",
            ],
            expected_distinct_trees: 4,
        },
        DistinctTreesTestCase {
            description: "a resource kind, a stored format and an EXTRACT field, bare and backquoted",
            written: &[
                "CREATE FUNCTION f1 AS 'a.B' USING JAR 'x.jar'",
                "CREATE FUNCTION f1 AS 'a.B' USING `JAR` 'x.jar'",
                "INSERT OVERWRITE DIRECTORY '/p' STORED AS TEXTFILE SELECT 1",
                "INSERT OVERWRITE DIRECTORY '/p' STORED AS `TEXTFILE` SELECT 1",
                "SELECT EXTRACT ( YEAR FROM v1 )",
                "SELECT EXTRACT ( `YEAR` FROM v1 )",
            ],
            expected_distinct_trees: 6,
        },
        DistinctTreesTestCase {
            description: "a tag key written as a name, backquoted, as a string and as a raw string",
            written: &[
                "ALTER TABLE t1 UNSET TAGS ( k )",
                "ALTER TABLE t1 UNSET TAGS ( `k` )",
                "ALTER TABLE t1 UNSET TAGS ( 'k' )",
                "ALTER TABLE t1 UNSET TAGS ( r'k' )",
                "ALTER TABLE t1 SET TAGS ( k = 'v' )",
                "ALTER TABLE t1 SET TAGS ( `k` = 'v' )",
                "ALTER TABLE t1 SET TAGS ( 'k' = 'v' )",
                "ALTER TABLE t1 SET TAGS ( r'k' = 'v' )",
            ],
            expected_distinct_trees: 8,
        },
        DistinctTreesTestCase {
            description: "WINDOW after a parenthesized query apart from WINDOW over a star of it",
            written: &[
                "( SELECT 1 LIMIT 1 ) WINDOW w AS ( )",
                "SELECT * FROM ( ( SELECT 1 LIMIT 1 ) ) WINDOW w AS ( )",
                "( SELECT 1 ) WINDOW w AS ( )",
                "SELECT * FROM ( ( SELECT 1 ) ) WINDOW w AS ( )",
                "( SELECT 1 WINDOW w AS ( ) )",
            ],
            expected_distinct_trees: 5,
        },
        DistinctTreesTestCase {
            description: "WINDOW after a parenthesized query apart from WINDOW inside it, the query \
                          able to take it",
            written: &[
                "( SELECT 1 ) ORDER BY 1 WINDOW w AS ( )",
                "( SELECT 1 ORDER BY 1 ) WINDOW w AS ( )",
                "WITH c AS ( SELECT 1 ) ( SELECT 1 ) WINDOW w AS ( )",
                "WITH c AS ( SELECT 1 ) ( SELECT 1 WINDOW w AS ( ) )",
                "WITH c AS ( SELECT 1 ) ( ( SELECT 1 ) ) WINDOW w AS ( )",
                "WITH c AS ( SELECT 1 ) ( ( SELECT 1 WINDOW w AS ( ) ) )",
            ],
            expected_distinct_trees: 6,
        },
        DistinctTreesTestCase {
            description: "a credential value as a string, a raw string and adjacent strings",
            written: &[
                "COPY INTO t1 FROM 's3://b/p' WITH ( CREDENTIAL ( k = '1x' ) ) FILEFORMAT = CSV",
                "COPY INTO t1 FROM 's3://b/p' WITH ( CREDENTIAL ( k = r'1x' ) ) FILEFORMAT = CSV",
                "COPY INTO t1 FROM 's3://b/p' WITH ( CREDENTIAL ( k = '1' 'x' ) ) FILEFORMAT = CSV",
            ],
            expected_distinct_trees: 3,
        },
        DistinctTreesTestCase {
            description: "a typed literal of a raw string apart from a plain one and from a cast",
            written: &[
                "SELECT DATE '2020-01-01'",
                "SELECT DATE r'2020-01-01'",
                "SELECT CAST ( r'2020-01-01' AS DATE )",
                "SELECT TIME '10:00:00'",
                "SELECT TIME r'10:00:00'",
                "SELECT TIMESTAMP '2020-01-01'",
                "SELECT TIMESTAMP r'2020-01-01'",
                "SELECT TIMESTAMP_NTZ '2020-01-01'",
                "SELECT TIMESTAMP_NTZ r'2020-01-01'",
                "SELECT X'AB'",
                "SELECT X r'AB'",
                "SELECT INTERVAL '1' DAY",
                "SELECT INTERVAL r'1' DAY",
                "SELECT INTERVAL - '1' DAY",
                "SELECT INTERVAL - r'1' DAY",
                "SELECT INTERVAL '1 day'",
                "SELECT INTERVAL r'1 day'",
            ],
            expected_distinct_trees: 17,
        },
        DistinctTreesTestCase {
            description: "adjacent strings apart from the one string they join to",
            written: &[
                "SELECT 'a' 'b'",
                "SELECT 'ab'",
                "SELECT r'a' 'b'",
                "SELECT 'a' 'b' 'c'",
                "SELECT 'ab' 'c'",
                "ALTER TABLE t1 SET TAGS ( 'a' 'b' = 'v' )",
                "ALTER TABLE t1 SET TAGS ( 'ab' = 'v' )",
            ],
            expected_distinct_trees: 7,
        },
        DistinctTreesTestCase {
            description: "a comment written as adjacent strings, as a raw string and as one string",
            written: &[
                "CREATE TABLE t1 ( c1 INT COMMENT 'a' 'b' )",
                "CREATE TABLE t1 ( c1 INT COMMENT r'ab' )",
                "CREATE TABLE t1 ( c1 INT COMMENT 'ab' )",
                "CREATE VIEW v1 COMMENT 'a' 'b' AS SELECT 1",
                "CREATE VIEW v1 COMMENT 'ab' AS SELECT 1",
                "CREATE VIEW v1 ( c1 COMMENT 'a' 'b' ) AS SELECT 1",
                "CREATE VIEW v1 ( c1 COMMENT 'ab' ) AS SELECT 1",
                "SELECT CAST ( NULL AS STRUCT < c1 : INT COMMENT 'a' 'b' > )",
                "SELECT CAST ( NULL AS STRUCT < c1 : INT COMMENT 'ab' > )",
                "ALTER TABLE t1 ADD COLUMN c1 INT COMMENT 'a' 'b'",
                "ALTER TABLE t1 ADD COLUMN c1 INT COMMENT 'ab'",
                "ALTER TABLE t1 ALTER COLUMN c1 COMMENT 'a' 'b'",
                "ALTER TABLE t1 ALTER COLUMN c1 COMMENT 'ab'",
                "ALTER TABLE t1 CHANGE COLUMN c1 c1 INT COMMENT 'a' 'b'",
                "ALTER TABLE t1 CHANGE COLUMN c1 c1 INT COMMENT 'ab'",
                "CREATE FUNCTION f1 ( p1 INT COMMENT 'a' 'b' ) RETURNS INT RETURN p1",
                "CREATE FUNCTION f1 ( p1 INT COMMENT r'ab' ) RETURNS INT RETURN p1",
                "CREATE FUNCTION f1 ( p1 INT COMMENT 'ab' ) RETURNS INT RETURN p1",
                "CREATE FUNCTION f1 ( p1 INT COMMENT 'b' ) RETURNS INT RETURN p1",
                "CREATE FUNCTION f1 ( p1 INT ) RETURNS INT RETURN p1",
            ],
            expected_distinct_trees: 20,
        },
        DistinctTreesTestCase {
            description: "a path, class, URI or state written as adjacent strings and as one string",
            written: &[
                "ALTER TABLE t1 SET LOCATION '/a' 'b'",
                "ALTER TABLE t1 SET LOCATION '/ab'",
                "ALTER TABLE t1 PARTITION ( c1 = 1 ) SET LOCATION '/a' 'b'",
                "ALTER TABLE t1 PARTITION ( c1 = 1 ) SET LOCATION '/ab'",
                "INSERT OVERWRITE DIRECTORY '/a' 'b' STORED AS ORC SELECT 1",
                "INSERT OVERWRITE DIRECTORY '/ab' STORED AS ORC SELECT 1",
                "INSERT OVERWRITE DIRECTORY '/p' STORED AS INPUTFORMAT 'a' 'b' OUTPUTFORMAT 'c' SELECT 1",
                "INSERT OVERWRITE DIRECTORY '/p' STORED AS INPUTFORMAT 'ab' OUTPUTFORMAT 'c' SELECT 1",
                "INSERT OVERWRITE DIRECTORY '/p' STORED AS INPUTFORMAT 'c' OUTPUTFORMAT 'a' 'b' SELECT 1",
                "INSERT OVERWRITE DIRECTORY '/p' STORED AS INPUTFORMAT 'c' OUTPUTFORMAT 'ab' SELECT 1",
                "LOAD DATA INPATH '/a' 'b' INTO TABLE t1",
                "LOAD DATA INPATH '/ab' INTO TABLE t1",
                "CREATE FUNCTION f1 AS 'a' 'b'",
                "CREATE FUNCTION f1 AS 'ab'",
                "CREATE FUNCTION f1 AS 'c' USING JAR '/a' 'b'",
                "CREATE FUNCTION f1 AS 'c' USING JAR '/ab'",
                "CREATE FUNCTION f1 ( ) RETURNS INT LANGUAGE PYTHON HANDLER 'a' 'b' AS $$ x $$",
                "CREATE FUNCTION f1 ( ) RETURNS INT LANGUAGE PYTHON HANDLER 'ab' AS $$ x $$",
                "BEGIN DECLARE c1 CONDITION FOR SQLSTATE '450' '00' ; END",
                "BEGIN DECLARE c1 CONDITION FOR SQLSTATE '45000' ; END",
                "BEGIN SIGNAL SQLSTATE '450' '00' ; END",
                "BEGIN SIGNAL SQLSTATE '45000' ; END",
                "USE CATALOG 'a' 'b'",
                "USE CATALOG 'ab'",
                "SHOW TBLPROPERTIES t1 ( 'a' 'b' )",
                "SHOW TBLPROPERTIES t1 ( 'ab' )",
                "CREATE TABLE t1 ( c1 INT ) USING DELTA OPTIONS ( 'a' 'b' = 1 )",
                "CREATE TABLE t1 ( c1 INT ) USING DELTA OPTIONS ( 'ab' = 1 )",
                "CREATE TABLE t1 ( c1 INT ) USING DELTA OPTIONS ( r'k' 'v' )",
                "CREATE TABLE t1 ( c1 INT ) USING DELTA OPTIONS ( 'k' 'v' )",
            ],
            expected_distinct_trees: 30,
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
