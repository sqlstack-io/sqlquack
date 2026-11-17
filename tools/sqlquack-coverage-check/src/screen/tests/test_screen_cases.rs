use serde_json::{Value, json};

use crate::screen::main::screen_cases::screen_cases;
use crate::screen::models::{AllowedSameTree, MutationKind, ScreenAllowlist, ScreenReport};
use crate::screen::tests::helpers::{FORM, case, databricks_tokens, databricks_tree, entry};
use crate::screen::tests::test_types::{AllowlistTestCase, KnownCollapseTestCase};

#[test]
fn given_a_spelling_the_grammar_keeps_when_screening_then_its_variant_has_its_own_tree() {
    let test_cases = [
        KnownCollapseTestCase {
            description: "control: text inside a comment is not read",
            sql: "SELECT /* m */ 1",
            variant: "SELECT /* */ 1",
            expected_same_tree: true,
        },
        KnownCollapseTestCase {
            description: "a TBLPROPERTIES value written without =",
            sql: "ALTER TABLE t1 SET TBLPROPERTIES ( 'k' = 'v' )",
            variant: "ALTER TABLE t1 SET TBLPROPERTIES ( 'k' 'v' )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "an OPTIONS key written as a name",
            sql: "CREATE TABLE t1 ( c1 INT ) USING DELTA OPTIONS ( 'k' = 'v' )",
            variant: "CREATE TABLE t1 ( c1 INT ) USING DELTA OPTIONS ( k = 'v' )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "REFERENCES without ON DELETE NO ACTION",
            sql: "CREATE TABLE t3 ( c1 INT REFERENCES t2 ON DELETE NO ACTION )",
            variant: "CREATE TABLE t3 ( c1 INT REFERENCES t2 )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "ON UPDATE written for ON DELETE",
            sql: "CREATE TABLE t3 ( c1 INT REFERENCES t2 ON DELETE NO ACTION )",
            variant: "CREATE TABLE t3 ( c1 INT REFERENCES t2 ON UPDATE NO ACTION )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a backquoted part of a qualified name",
            sql: "SHOW TABLES IN s1 . v1",
            variant: "SHOW TABLES IN `s1` . v1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "TEMP for TEMPORARY",
            sql: "CREATE TEMPORARY FUNCTION f1 ( ) RETURNS INT RETURN 1",
            variant: "CREATE TEMP FUNCTION f1 ( ) RETURNS INT RETURN 1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "GLOBAL TEMP for GLOBAL TEMPORARY",
            sql: "CREATE GLOBAL TEMPORARY VIEW v1 AS SELECT 1",
            variant: "CREATE GLOBAL TEMP VIEW v1 AS SELECT 1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a CTE written without AS",
            sql: "WITH w1 AS ( SELECT 1 ) SELECT 1",
            variant: "WITH w1 ( SELECT 1 ) SELECT 1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "ALTER VIEW written without AS",
            sql: "ALTER VIEW v1 AS SELECT 1",
            variant: "ALTER VIEW v1 SELECT 1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a backquoted COPY INTO file format",
            sql: "COPY INTO t1 FROM '/tmp/x' FILEFORMAT = PARQUET",
            variant: "COPY INTO t1 FROM '/tmp/x' FILEFORMAT = `PARQUET`",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a backquoted CREDENTIAL name",
            sql: "COPY INTO t1 WITH ( CREDENTIAL v1 ) FROM '/tmp/x' FILEFORMAT = PARQUET",
            variant: "COPY INTO t1 WITH ( CREDENTIAL `v1` ) FROM '/tmp/x' FILEFORMAT = PARQUET",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a backquoted INSERT OVERWRITE DIRECTORY format",
            sql: "INSERT OVERWRITE DIRECTORY '/tmp/x' USING PARQUET SELECT 1",
            variant: "INSERT OVERWRITE DIRECTORY '/tmp/x' USING `PARQUET` SELECT 1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a backquoted RESET key",
            sql: "RESET v1",
            variant: "RESET `v1`",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a backquoted function name",
            sql: "SELECT abs ( 1 )",
            variant: "SELECT `abs` ( 1 )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a DBPROPERTIES value written without =",
            sql: "CREATE SCHEMA s1 WITH DBPROPERTIES ( 'k' = 'v' )",
            variant: "CREATE SCHEMA s1 WITH DBPROPERTIES ( 'k' 'v' )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a connection OPTIONS value written without =",
            sql: "CREATE CONNECTION c1 TYPE mysql OPTIONS ( host = 'h' )",
            variant: "CREATE CONNECTION c1 TYPE mysql OPTIONS ( host 'h' )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a COPY INTO format option written without =",
            sql: "COPY INTO t1 FROM '/tmp/x' FILEFORMAT = PARQUET FORMAT_OPTIONS ( 'k' = 'v' )",
            variant: "COPY INTO t1 FROM '/tmp/x' FILEFORMAT = PARQUET FORMAT_OPTIONS ( 'k' 'v' )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a COPY INTO copy option written without =",
            sql: "COPY INTO t1 FROM '/tmp/x' FILEFORMAT = PARQUET COPY_OPTIONS ( 'k' = 'v' )",
            variant: "COPY INTO t1 FROM '/tmp/x' FILEFORMAT = PARQUET COPY_OPTIONS ( 'k' 'v' )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a table alias written with AS",
            sql: "SELECT 1 FROM t1 a1",
            variant: "SELECT 1 FROM t1 AS a1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "control: a string written in double quotes is the same literal",
            sql: "SELECT 'v'",
            variant: "SELECT \"v\"",
            expected_same_tree: true,
        },
        KnownCollapseTestCase {
            description: "a LATERAL function's alias written with AS",
            sql: "SELECT * FROM LATERAL range ( 1 ) a1",
            variant: "SELECT * FROM LATERAL range ( 1 ) AS a1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a VALUES alias written with AS",
            sql: "VALUES 1 , 2 a1",
            variant: "VALUES 1 , 2 AS a1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a FROM VALUES alias written with AS",
            sql: "SELECT * FROM VALUES ( 1 ) a1",
            variant: "SELECT * FROM VALUES ( 1 ) AS a1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a PIVOT alias written with AS",
            sql: "SELECT * FROM t1 PIVOT ( SUM ( c1 ) FOR c2 IN ( 1 ) ) a1",
            variant: "SELECT * FROM t1 PIVOT ( SUM ( c1 ) FOR c2 IN ( 1 ) ) AS a1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "an UNPIVOT alias written with AS",
            sql: "SELECT * FROM t1 UNPIVOT ( c1 FOR c2 IN ( c3 ) ) a1",
            variant: "SELECT * FROM t1 UNPIVOT ( c1 FOR c2 IN ( c3 ) ) AS a1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "! written for a prefix NOT",
            sql: "SELECT NOT 1",
            variant: "SELECT ! 1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "WITH PROPERTIES written for WITH DBPROPERTIES",
            sql: "CREATE SCHEMA s1 WITH DBPROPERTIES ( 'k' = 'v' )",
            variant: "CREATE SCHEMA s1 WITH PROPERTIES ( 'k' = 'v' )",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "hints written without the comma between them",
            sql: "SELECT /*+ REPARTITION ( 1 ) , COALESCE ( 1 ) */ 1",
            variant: "SELECT /*+ REPARTITION ( 1 ) COALESCE ( 1 ) */ 1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a backquoted hint name",
            sql: "SELECT /*+ BROADCAST ( t1 ) */ 1",
            variant: "SELECT /*+ `BROADCAST` ( t1 ) */ 1",
            expected_same_tree: false,
        },
        KnownCollapseTestCase {
            description: "a LATERAL VIEW written without AS",
            sql: "SELECT * FROM t1 LATERAL VIEW explode ( array ( 1 ) ) a3 AS x1",
            variant: "SELECT * FROM t1 LATERAL VIEW explode ( array ( 1 ) ) a3 x1",
            expected_same_tree: false,
        },
    ];
    for test_case in test_cases {
        let original: Option<Value> = databricks_tree(test_case.sql);
        let variant: Option<Value> = databricks_tree(test_case.variant);
        assert!(
            original.is_some(),
            "{}: the text parses",
            test_case.description
        );
        assert!(
            variant.is_some(),
            "{}: the variant parses",
            test_case.description
        );
        assert_eq!(
            original == variant,
            test_case.expected_same_tree,
            "{}",
            test_case.description
        );
        let report: ScreenReport = screen_cases(
            &[case(test_case.sql)],
            (&databricks_tree, &databricks_tokens),
            &ScreenAllowlist::default(),
            true,
        );
        let flagged: bool = report
            .flagged
            .iter()
            .any(|found| found.mutation.sql == test_case.variant);
        assert_eq!(
            flagged, test_case.expected_same_tree,
            "{}: the screen flags the variant",
            test_case.description
        );
    }
}

#[test]
fn given_an_allowlist_when_screening_then_only_unlisted_same_trees_fail_and_unused_entries_are_stale()
 {
    let constant = |sql: &str| -> Option<Value> {
        (!sql.contains(['\'', '`', '"', '=']) && !sql.split(' ').any(|word| word == "AS"))
            .then(|| json!(["one tree"]))
    };
    let test_cases = [
        AllowlistTestCase {
            description: "no entry: both one-token drops that parse alike are flagged",
            entries: vec![],
            full: true,
            expected_flagged: 2,
            expected_allowed: 0,
            expected_stale: 0,
            expected_unpinned: 0,
        },
        AllowlistTestCase {
            description: "an entry for one token allows only that token",
            entries: vec![entry(FORM, MutationKind::Drop, Some("m2"))],
            full: true,
            expected_flagged: 1,
            expected_allowed: 1,
            expected_stale: 0,
            expected_unpinned: 0,
        },
        AllowlistTestCase {
            description: "an entry pinned to neither a token nor untokenized text fails as unpinned",
            entries: vec![entry(FORM, MutationKind::Drop, None)],
            full: true,
            expected_flagged: 0,
            expected_allowed: 2,
            expected_stale: 0,
            expected_unpinned: 1,
        },
        AllowlistTestCase {
            description: "an untokenized entry does not allow a change to a lexer token",
            entries: vec![AllowedSameTree {
                untokenized: true,
                ..entry(FORM, MutationKind::Drop, None)
            }],
            full: true,
            expected_flagged: 2,
            expected_allowed: 0,
            expected_stale: 1,
            expected_unpinned: 0,
        },
        AllowlistTestCase {
            description: "an entry no same-tree variant matches is stale",
            entries: vec![entry(FORM, MutationKind::Drop, Some("m3"))],
            full: true,
            expected_flagged: 2,
            expected_allowed: 0,
            expected_stale: 1,
            expected_unpinned: 0,
        },
        AllowlistTestCase {
            description: "an entry for the last token allows only the last token's drop",
            entries: vec![AllowedSameTree {
                last_token: true,
                ..entry(FORM, MutationKind::Drop, Some("m2"))
            }],
            full: true,
            expected_flagged: 1,
            expected_allowed: 1,
            expected_stale: 0,
            expected_unpinned: 0,
        },
        AllowlistTestCase {
            description: "on a full run an entry for a form with no screened case is stale",
            entries: vec![entry("db.ddl.other", MutationKind::Drop, Some("m1"))],
            full: true,
            expected_flagged: 2,
            expected_allowed: 0,
            expected_stale: 1,
            expected_unpinned: 0,
        },
        AllowlistTestCase {
            description: "on a run of chosen forms an entry for another form is not judged",
            entries: vec![entry("db.ddl.other", MutationKind::Drop, Some("m1"))],
            full: false,
            expected_flagged: 2,
            expected_allowed: 0,
            expected_stale: 0,
            expected_unpinned: 0,
        },
    ];
    for test_case in test_cases {
        let report: ScreenReport = screen_cases(
            &[case("m1 m2")],
            (&constant, &|sql: &str| vec![(0, 2), (3, sql.len())]),
            &ScreenAllowlist {
                lexical: vec![],
                allowed: test_case.entries,
            },
            test_case.full,
        );
        assert_eq!(
            (
                report.flagged.len(),
                report.allowed.len(),
                report.stale.len(),
                report.unpinned.len()
            ),
            (
                test_case.expected_flagged,
                test_case.expected_allowed,
                test_case.expected_stale,
                test_case.expected_unpinned
            ),
            "{}",
            test_case.description
        );
    }
}
