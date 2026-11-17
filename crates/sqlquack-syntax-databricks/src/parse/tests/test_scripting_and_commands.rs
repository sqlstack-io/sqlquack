//! Verdicts of compound statements, tags, file commands and IDENTIFIER clauses, as the warehouse gave them.

use crate::parse::main::parse::parse;
use crate::parse::tests::test_types::{ExpectedVerdict, VerdictTestCase};

#[test]
fn given_recorded_statements_when_parsing_then_the_verdict_matches_the_warehouse() {
    let test_cases = [
        VerdictTestCase {
            description: "a FOR query ends at DO, which names no alias",
            sql: "BEGIN FOR SELECT 1 DO SELECT 1 ; END FOR ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a FOR query over a derived table ends at DO",
            sql: "BEGIN FOR SELECT * FROM ( SELECT 1 ) DO SELECT 1 ; END FOR ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a FOR statement needs DO",
            sql: "BEGIN FOR SELECT 1 SELECT 1 ; END FOR ; END",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an ATOMIC compound statement is checked",
            sql: "BEGIN ATOMIC SELECT 1 ; SELECT 2 ; END ;",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "an ATOMIC compound body holds statements",
            sql: "BEGIN ATOMIC 1 ; END",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "GET DIAGNOSTICS reads a statement item",
            sql: "BEGIN DECLARE v1 INT ; DECLARE EXIT HANDLER FOR SQLEXCEPTION BEGIN GET DIAGNOSTICS v1 = ROW_COUNT , v1 = TRANSACTION_ACTIVE ; END ; SELECT 1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "GET DIAGNOSTICS CONDITION 1 reads a condition item",
            sql: "BEGIN DECLARE EXIT HANDLER FOR SQLEXCEPTION BEGIN DECLARE v1 STRING ; GET DIAGNOSTICS CONDITION 1 v1 = MESSAGE_TEXT ; END ; SELECT 1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a condition item needs CONDITION 1",
            sql: "BEGIN DECLARE EXIT HANDLER FOR SQLEXCEPTION BEGIN GET DIAGNOSTICS v1 = MESSAGE_TEXT ; END ; SELECT 1 ; END",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "GET DIAGNOSTICS CONDITION takes a number",
            sql: "BEGIN DECLARE EXIT HANDLER FOR SQLEXCEPTION BEGIN GET DIAGNOSTICS CONDITION v1 v1 = MESSAGE_TEXT ; END ; SELECT 1 ; END",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER CATALOG tag keys are strings",
            sql: "ALTER CATALOG v1 SET TAGS ( 'k' = 'v' , 'k2' = 'v' )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ALTER CATALOG refuses a name as a tag key",
            sql: "ALTER CATALOG UNSET TAGS ( m1 )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER SCHEMA tag keys are strings",
            sql: "ALTER SCHEMA s1 UNSET TAGS ( 'k' , 'k2' )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ALTER SCHEMA refuses a name as a tag key",
            sql: "ALTER DATABASE s1 SET TAGS ( m1 = 'v' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "governed tag values are strings",
            sql: "CREATE GOVERNED TAG v1 DESCRIPTION 'x' VALUES ( 'm' , 'm2' )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "governed tag values refuse a name",
            sql: "ALTER GOVERNED TAG v1 SET VALUES ( m1 )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "column tags apply to a top-level column",
            sql: "ALTER TABLE t1 ALTER COLUMN c1.c1 SET TAGS ( 'k' = 'v' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a column mask applies to a top-level column",
            sql: "ALTER TABLE t1 ALTER c1.c1 SET MASK f1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "FSCK takes its mode before DRY RUN",
            sql: "FSCK REPAIR TABLE t1 METADATA ONLY DRY RUN",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SHOW TABLES DROPPED takes a LIMIT",
            sql: "SHOW TABLES DROPPED FROM s1 LIMIT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "GET copies a volume path to a local path",
            sql: "GET '/Volumes/workspace/s1/v1/f1.csv' TO '/tmp/f1.csv'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "GET needs TO",
            sql: "GET '/Volumes/workspace/s1/v1/f1.csv' '/tmp/f1.csv'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "PUT copies a local path into a volume path",
            sql: "PUT '/tmp/f1.csv' INTO '/Volumes/workspace/s1/v1/f1.csv' OVERWRITE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "PUT takes string paths",
            sql: "PUT m1 INTO '/Volumes/workspace/s1/v1/f1.csv'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an IDENTIFIER clause names a function to call",
            sql: "SELECT IDENTIFIER ( 'v1' ) ( )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "an IDENTIFIER clause names a procedure to call",
            sql: "CALL IDENTIFIER ( 'v1' ) ( )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an IDENTIFIER clause names a label",
            sql: "BEGIN IDENTIFIER ( 'v1' ) : BEGIN SELECT 1 ; END ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "an IDENTIFIER clause names the schema SHOW TABLES lists",
            sql: "SHOW TABLES IN IDENTIFIER ( 'v1' )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a JVM handler is a string after its ENVIRONMENT",
            sql: "CREATE FUNCTION abs ( ) RETURNS INT LANGUAGE SCALA ENVIRONMENT ( java_dependencies = '[\"/Volumes/c1/s1/v1/u1.jar\"]' , environment_version = '4' ) HANDLER 'm'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a handler is not a name",
            sql: "CREATE FUNCTION abs ( ) RETURNS INT LANGUAGE SCALA ENVIRONMENT ( java_dependencies = '[\"/Volumes/c1/s1/v1/u1.jar\"]' , environment_version = '4' ) HANDLER m1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a Scala handler needs an ENVIRONMENT",
            sql: "CREATE FUNCTION abs ( ) RETURNS INT LANGUAGE SCALA HANDLER 'm'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a Python handler is a string before its body",
            sql: "CREATE FUNCTION abs ( ) RETURNS INT LANGUAGE PYTHON HANDLER 'm' AS $$ return 1 $$",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a clone source takes an @ version",
            sql: "CREATE TABLE t3 CLONE t1@v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a clone source takes an @ timestamp",
            sql: "CREATE TABLE t3 CLONE t1@20261101000000000",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an @ version is written against the table name",
            sql: "CREATE TABLE t3 CLONE t1 @v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an @ timestamp has seventeen digits",
            sql: "CREATE TABLE t3 CLONE t1@2026",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a recipient's sharing id is a string",
            sql: "CREATE RECIPIENT v1 USING ID 'm'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a recipient's sharing id is not a name",
            sql: "CREATE RECIPIENT v1 USING ID m1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a group's ADD names several members",
            sql: "ALTER GROUP v1 ADD GROUP v1 , v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SET CATALOG with a name parses (recI: NO_SUCH_CATALOG_EXCEPTION, Catalog 'v1' was not found)",
            sql: "SET CATALOG v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a catalog given an owner is named, not quoted",
            sql: "ALTER CATALOG 'v1' OWNER TO v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a catalog given tags is named, not quoted",
            sql: "ALTER CATALOG 'v1' SET TAGS ( 'k' = 'v' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a TABLE argument missing its ) is refused at the end, not at its comma",
            sql: "SELECT * FROM AI_PREDICT_VALUE ( input => TABLE ( v1 , target_col => v1 , feature_cols => v1 )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a catalog given predictive optimization is named, not quoted",
            sql: "ALTER CATALOG 'v1' ENABLE PREDICTIVE OPTIMIZATION",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a catalog's tags are unset by name, not by a quoted name",
            sql: "ALTER CATALOG 'v1' UNSET TAGS ( 'k' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "each query tag names QUERY_TAGS and a key",
            sql: "SET QUERY_TAGS [ 'k' ] = 'v' , QUERY_TAGS [ 'k2' ] = UNSET",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a query tag needs a key",
            sql: "SET QUERY_TAGS [ ] = 'v'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a later query tag repeats QUERY_TAGS",
            sql: "SET QUERY_TAGS [ 'k' ] = 'v' , [ 'k2' ] = 'v'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an inventory location needs CONF",
            sql: "ANALYZE TABLE t1 COMPUTE STORAGE METRICS USING INVENTORY LOCATION 'm'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "adjacent strings join into one inventory location, which still needs CONF",
            sql: "ANALYZE TABLE t1 COMPUTE STORAGE METRICS USING INVENTORY LOCATION 'm' 'v1'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an inventory location of adjacent strings takes CONF",
            sql: "ANALYZE TABLE t1 COMPUTE STORAGE METRICS USING INVENTORY LOCATION 'm' 'v1' CONF 'v1'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a procedure's compound body may follow without AS",
            sql: "CREATE PROCEDURE v1 ( ) LANGUAGE SQL SQL SECURITY INVOKER BEGIN SELECT 1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a procedure's other body needs AS",
            sql: "CREATE PROCEDURE v1 ( ) LANGUAGE SQL SQL SECURITY INVOKER SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an external location's STORAGE before CREDENTIAL is optional",
            sql: "CREATE EXTERNAL LOCATION v1 URL 's3://b1/p1' WITH ( CREDENTIAL v1 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a do alias inside a FOR query's subquery is still an alias",
            sql: "BEGIN FOR r AS SELECT * FROM ( SELECT * FROM t1 do ) DO SELECT 1 ; END FOR ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a label named atomic is a label",
            sql: "BEGIN atomic : BEGIN SELECT 1 ; END ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a procedure body without AS is a compound statement, not a transaction",
            sql: "CREATE PROCEDURE v1 ( ) LANGUAGE SQL SQL SECURITY INVOKER BEGIN TRANSACTION",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a FILE column is managed or external",
            sql: "CREATE TABLE t3 ( c1 FILE EXTERNAL , c2 FILE MANAGED )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a FILE column names its kind",
            sql: "CREATE TABLE t3 ( c1 FILE )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unqualified interval string names its units",
            sql: "SELECT INTERVAL '1 day 2 hours'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an unqualified interval string without units is refused",
            sql: "SELECT ( INTERVAL '1-2' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a refresh policy names its mode",
            sql: "CREATE MATERIALIZED VIEW mv1 REFRESH POLICY INCREMENTAL STRICT AS SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a refresh policy without a mode is refused",
            sql: "CREATE MATERIALIZED VIEW mv1 REFRESH POLICY AS SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "REORG applies a checkpoint",
            sql: "REORG t1 APPLY ( CHECKPOINT )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REORG sets the Parquet format version",
            sql: "REORG t1 APPLY ( SET PARQUET ( FORMAT_VERSION = '2.12.0' ) )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a policy target may be a bare name",
            sql: "CREATE POLICY v1 ON t1 ROW FILTER f1 TO `u1` FOR TABLES",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a policy still needs a target",
            sql: "CREATE POLICY v1 ON ROW FILTER f1 TO `u1` FOR TABLES",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "GRANT on an unnamed catalog parses (UC_CATALOG_NAME_NOT_PROVIDED: please specify the catalog name explicitly)",
            sql: "GRANT SELECT ON CATALOG TO `u1`",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "GRANT on the metastore parses (PRINCIPAL_DOES_NOT_EXIST: Could not find principal with name u1)",
            sql: "GRANT SELECT ON METASTORE TO `u1`",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a bare LOCALTIME parses (accepted)",
            sql: "SELECT ( LOCALTIME )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a CACHE TABLE option without = parses (NOT_SUPPORTED_WITH_DB_SQL: CACHE TABLE is not supported on a SQL warehouse)",
            sql: "CACHE TABLE t1 OPTIONS ( 'storageLevel' 'DISK_ONLY' )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "RETAIN DROPPED TO a count and unit parses (SCHEMA_NOT_FOUND: The schema `workspace.s1` cannot be found)",
            sql: "ALTER DATABASE s1 RETAIN DROPPED TO 1 HOUR",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "RETAIN DROPPED FOR needs its unit (missing {'DAY', 'DAYS', 'HOUR', 'HOURS', 'MINUTE', 'MINUTES', 'WEEK', 'WEEKS'})",
            sql: "CREATE CATALOG v1 RETAIN DROPPED FOR 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a BD literal with an exponent parses (accepted)",
            sql: "SELECT ( 12E-1BD )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "TIMESTAMP_LTZ with a precision parses (accepted)",
            sql: "SELECT CAST ( NULL AS TIMESTAMP_LTZ ( 6 ) )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a Hive GROUPING SETS after the GROUP BY list parses (accepted)",
            sql: "SELECT c1 FROM t1 GROUP BY c1 GROUPING SETS ( c1 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "INSERT WITH SCHEMA EVOLUTION INTO TABLE parses (INSERT_COLUMN_ARITY_MISMATCH.NOT_ENOUGH_DATA_COLUMNS)",
            sql: "WITH w1 AS ( SELECT 1 ) INSERT WITH SCHEMA EVOLUTION INTO TABLE t1 ( c1 , c2 ) REPLACE WHERE c1 = 1 SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REPLACE ON with a parenthesised query parses (DELTA_INSERT_COLUMN_ARITY_MISMATCH)",
            sql: "INSERT INTO t1 REPLACE ON 1 = 1 ( SELECT 1 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REPLACE USING parses (DELTA_INSERT_COLUMN_ARITY_MISMATCH)",
            sql: "INSERT INTO t1 REPLACE USING ( c1 ) SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MERGE WITH SCHEMA EVOLUTION after a CTE parses (accepted)",
            sql: "WITH w1 AS ( SELECT 1 ) MERGE WITH SCHEMA EVOLUTION INTO t1 a1 USING t2 a2 ON a1.c1 = a2.c1 WHEN MATCHED AND a1.c2 = 2 THEN DELETE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "COPY INTO BY POSITION parses (DBFS_DISABLED, a runtime error)",
            sql: "COPY INTO t1 BY POSITION FROM '/tmp/x' FILEFORMAT = PARQUET",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "COPY INTO with a target credential parses (WITH_CREDENTIAL: not supported for table identifiers)",
            sql: "COPY INTO t1 WITH ( CREDENTIAL v1 ) FROM '/tmp/x' FILEFORMAT = PARQUET",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "INSERT OVERWRITE DIRECTORY with USING parses (DBFS_DISABLED, a runtime error)",
            sql: "INSERT OVERWRITE DIRECTORY '/tmp/x' USING PARQUET SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a directory named by its path option parses (DBFS_DISABLED, a runtime error)",
            sql: "INSERT OVERWRITE DIRECTORY USING PARQUET OPTIONS ( 'path' '/tmp/x' ) SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "APPROX NEAREST BY DISTANCE parses (DATATYPE_MISMATCH, a semantic error)",
            sql: "SELECT * FROM t1 JOIN t2 APPROX NEAREST BY DISTANCE vector_l2_distance ( 1 , 1 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a sampled LATERAL subquery parses (accepted)",
            sql: "SELECT * FROM LATERAL ( SELECT 1 ) TABLESAMPLE ( 50 PERCENT ) a1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a sampled table with a WATERMARK parses (accepted)",
            sql: "SELECT * FROM t1 TABLESAMPLE ( 50 PERCENT ) WATERMARK c1 DELAY OF INTERVAL 1 MINUTE a1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an aliased parenthesized STREAM parses (streaming relations unsupported here)",
            sql: "SELECT * FROM STREAM ( t1 ) a1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ADD of a file is checked",
            sql: "ADD FILE v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "LIST of jars may name none",
            sql: "LIST JAR",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ADD needs a resource kind before a string",
            sql: "ADD 'f1.parquet'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ADD refuses an unclosed string among its resources",
            sql: "ADD FILE 'f1.parquet",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ADD keeps resources its form cannot split as a gap",
            sql: "ADD FILE ( f1 ; f2 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "RESTORE takes a table named by IDENTIFIER",
            sql: "RESTORE IDENTIFIER ( 't1' ) VERSION AS OF 0",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "RESTORE TABLE needs a table name",
            sql: "RESTORE TABLE VERSION AS OF 0",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CACHE SELECT refuses a string after any FROM, even inside an expression",
            sql: "CACHE SELECT trim ( BOTH FROM 'x' ) FROM t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CREATE TABLE keeps a Hive row format and storage format",
            sql: "CREATE TABLE t3 ( c1 INT ) ROW FORMAT DELIMITED STORED AS TEXTFILE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE LIKE keeps a Hive row format",
            sql: "CREATE TABLE t4 LIKE t3 ROW FORMAT SERDE 's1' STORED AS INPUTFORMAT 'i1' OUTPUTFORMAT 'o1'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a storage handler's SERDEPROPERTIES, which no node holds, stay a gap",
            sql: "CREATE TABLE t3 STORED BY 'h1' WITH SERDEPROPERTIES ( 'k' = 'v' )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a key constraint keeps ENABLE NOVALIDATE",
            sql: "CREATE TABLE t3 ( c1 INT , CONSTRAINT k1 PRIMARY KEY ( c1 ) ENABLE NOVALIDATE )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an added constraint keeps DISABLE VALIDATE",
            sql: "ALTER TABLE t3 ADD CONSTRAINT k1 CHECK ( c1 > 0 ) DISABLE VALIDATE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "validation words out of their order, which no modifier holds, stay a gap",
            sql: "ALTER TABLE t3 ADD CONSTRAINT k1 CHECK ( c1 > 0 ) VALIDATE ENABLE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a repeated validation word stays a gap",
            sql: "CREATE TABLE t3 ( c1 INT , PRIMARY KEY ( c1 ) NOVALIDATE NOVALIDATE )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "RESTORE is checked against its syntax",
            sql: "RESTORE TABLE t1 TO VERSION AS OF 0",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "RESTORE needs a version or timestamp",
            sql: "RESTORE t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DENY is checked",
            sql: "DENY SELECT , MODIFY ON TABLE t1 TO v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DENY needs TO",
            sql: "DENY ALL PRIVILEGES ON TABLE t1 v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CACHE SELECT keeps text the service reads loosely as a gap",
            sql: "CACHE SELECT c1 FROM",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "CACHE SELECT keeps an opaque part as a gap",
            sql: "CACHE SELECT IDENTIFIER ( 'f1' ) ( 1 ) FROM t1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "CACHE SELECT as its page writes it is checked",
            sql: "CACHE SELECT c1 FROM t1 WHERE c1 = 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CACHE SELECT reads no string as its table",
            sql: "CACHE SELECT c1 FROM 'v1'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "REFRESH of a table name is a resource path",
            sql: "REFRESH t1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DESCRIBE of one column is checked",
            sql: "DESCRIBE TABLE t1 c1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SHOW PARTITIONS of one partition is checked",
            sql: "SHOW PARTITIONS t1 PARTITION ( c1 = 1 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an owner change is checked",
            sql: "ALTER TABLE t1 OWNER TO u1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a row filter set on a table is checked",
            sql: "ALTER TABLE t1 SET ROW FILTER f1 ON ( )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a row filter needs the columns passed to it",
            sql: "ALTER TABLE t1 SET ROW FILTER f1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET MANAGED TRUNCATE needs UNIFORM HISTORY",
            sql: "ALTER TABLE t1 SET MANAGED TRUNCATE HISTORY",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET EXTERNAL DRY RUN is checked",
            sql: "ALTER TABLE t1 SET EXTERNAL DRY RUN",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a dropped foreign key is checked",
            sql: "ALTER TABLE t1 DROP FOREIGN KEY IF EXISTS ( c1 , c2 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "each column of a list keeps its own action",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 FIRST , c2 DROP MASK",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a column with no action after a kept one is accepted as a gap, since no tree holds it",
            sql: "ALTER TABLE t1 ALTER COLUMN c1 FIRST , c2",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a column with no action is refused",
            sql: "ALTER TABLE t1 ALTER COLUMN c1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a column with no action after a kept form earlier in its block is accepted, as after a gap",
            sql: "BEGIN ALTER TABLE t1 OWNER TO u1 ; ALTER TABLE t1 ALTER COLUMN c1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a column with no action after a kept Hive format earlier in its block is accepted, as after a gap",
            sql: "BEGIN CREATE TABLE t2 ( c1 INT ) STORED AS PARQUET ; ALTER TABLE t1 ALTER COLUMN c1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a column with no action after RESTORE earlier in its block is accepted, as after a gap",
            sql: "BEGIN RESTORE t2 VERSION AS OF 1 ; ALTER TABLE t1 ALTER COLUMN c1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a column with no action after a kept form in an earlier statement is refused",
            sql: "ALTER TABLE t1 OWNER TO u1 ; ALTER TABLE t1 ALTER COLUMN c1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "UNSET TBLPROPERTIES IF EXISTS stays a gap",
            sql: "ALTER TABLE t1 UNSET TBLPROPERTIES IF EXISTS ( 'k' )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a directory whose row format escapes stays a gap",
            sql: "INSERT OVERWRITE DIRECTORY '/tmp/x' ROW FORMAT DELIMITED FIELDS TERMINATED BY ',' ESCAPED BY '#' SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a directory with a SerDe stays a gap",
            sql: "INSERT OVERWRITE DIRECTORY '/tmp/x' ROW FORMAT SERDE 's1' SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a directory stored by a handler stays a gap",
            sql: "INSERT OVERWRITE DIRECTORY '/tmp/x' STORED BY 'h1' SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a Hive directory is kept",
            sql: "INSERT OVERWRITE DIRECTORY '/tmp/x' STORED AS TEXTFILE VALUES ( 1 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "COPY INTO columns are kept",
            sql: "COPY INTO t1 ( c1 , c2 ) FROM '/tmp/x' FILEFORMAT = CSV",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
    ];
    for test_case in test_cases {
        let verdict: ExpectedVerdict = parse(test_case.sql)
            .map(|parsed| ExpectedVerdict::Accepted {
                opaque: !parsed.gaps.is_empty(),
            })
            .unwrap_or_else(|error| ExpectedVerdict::Rejected {
                syntax: error.syntax,
            });
        assert_eq!(
            verdict, test_case.expected_verdict,
            "{}",
            test_case.description
        );
    }
}
