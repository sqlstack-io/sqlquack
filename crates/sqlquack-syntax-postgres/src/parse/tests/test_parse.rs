//! Verdicts and trees of the PostgreSQL parser, each confirmed against PostgreSQL 18.6.

use serde_json::Value;

use crate::parse::main::parse::parse;
use crate::parse::tests::test_types::{ExpectedVerdict, ShapeTestCase, VerdictTestCase};

#[test]
fn given_sql_when_parsing_then_the_verdict_is_postgresqls() {
    let test_cases = [
        VerdictTestCase {
            description: "an unknown table, which only analysis rejects",
            sql: "SELECT a, b FROM missing_t WHERE a > 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a keyword usable as a bare label",
            sql: "SELECT 1 like FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a generic type name before a string, as a typed literal",
            sql: "SELECT a 'x' FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a column-name keyword as a column",
            sql: "SELECT time FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a reserved keyword after AS",
            sql: "SELECT 1 AS from",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "IS NULL repeated",
            sql: "SELECT a IS NULL IS NULL",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "array types after :: and ARRAY with a bound",
            sql: "SELECT x::int[], '{1}'::int ARRAY[2]",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a join nested by its qualifiers",
            sql: "SELECT * FROM a JOIN b JOIN c ON true ON true",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a value function called with parentheses",
            sql: "SELECT current_schema()",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "WITH before a set operation with ORDER BY and LIMIT",
            sql: "WITH r AS MATERIALIZED (SELECT 1) SELECT * FROM r UNION SELECT 2 ORDER BY 1 LIMIT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "INSERT with an alias, ON CONFLICT and RETURNING",
            sql: "INSERT INTO t AS x (a) VALUES (1) ON CONFLICT (a) DO UPDATE SET a = excluded.a RETURNING *",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "UPDATE with FROM",
            sql: "UPDATE t SET a = 1 FROM s WHERE t.id = s.id",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DELETE with an alias and USING",
            sql: "DELETE FROM t u USING s WHERE u.id = s.id",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a semicolon inside a dollar quote",
            sql: "SELECT $$a;b$$; SELECT 2",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "OFFSET and FETCH in their standard spellings",
            sql: "SELECT 1 OFFSET 2 ROWS FETCH FIRST ROW ONLY",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "named arguments in both spellings",
            sql: "SELECT f(a => 1, b := 2)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an aggregate with FILTER and a framed window",
            sql: "SELECT count(*) FILTER (WHERE x) OVER (PARTITION BY a ROWS BETWEEN 1 PRECEDING AND CURRENT ROW EXCLUDE TIES) FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "five dotted names as a field selection",
            sql: "SELECT a.b.c.d.e FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE, which the parser keeps opaque",
            sql: "CREATE TABLE t (a int)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "MERGE, which the parser keeps opaque",
            sql: "MERGE INTO t USING s ON true WHEN MATCHED THEN DELETE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a comparison chained",
            sql: "SELECT a = b = c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "LIKE chained",
            sql: "SELECT a like b like c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a type-or-function keyword as a column",
            sql: "SELECT left FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a keyword that is not a bare label",
            sql: "SELECT 1 year",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a field of a call's result without parentheses",
            sql: "SELECT f(1).x",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a subscript on a constant",
            sql: "SELECT 'x'[1]",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ARRAY with empty brackets",
            sql: "SELECT CAST(NULL AS INT ARRAY [ ])",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "WITH TIES without ORDER BY",
            sql: "SELECT 1 FETCH FIRST ROWS WITH TIES",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "LIMIT with a comma",
            sql: "SELECT 1 LIMIT 1, 2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a parenthesized table that is not a join",
            sql: "SELECT * FROM (t)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a statement starting with FROM",
            sql: "FROM ONLY t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "AT TIME without ZONE",
            sql: "SELECT x at time",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a second ORDER BY around a parenthesized query",
            sql: "(SELECT 1 ORDER BY 1) ORDER BY 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a table name with four parts",
            sql: "SELECT 1 FROM a.b.c.d",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unterminated string",
            sql: "SELECT 'abc",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "the UNIQUE predicate, refused as unsupported",
            sql: "SELECT UNIQUE (SELECT 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a malformed Unicode escape in an escape string",
            sql: "SELECT E'\\u12'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "COPY options in parentheses with a star argument",
            sql: "COPY t TO STDOUT WITH (FORMAT csv, FORCE_QUOTE *)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "old-style COPY options with a bare column list",
            sql: "COPY t (a, b) FROM STDIN WITH CSV HEADER FORCE NOT NULL a, b",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "COPY FROM PROGRAM with STDIN",
            sql: "COPY t FROM PROGRAM STDIN",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a number inside a COPY option's list",
            sql: "COPY t FROM 'f' (a (1))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "COPY of a query FROM a file",
            sql: "COPY (SELECT 1) FROM STDIN",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "COPY TO with a WHERE clause",
            sql: "COPY t TO 'f' WHERE a > 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "USING without DELIMITERS in COPY",
            sql: "COPY t FROM 'f' USING ','",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "TRUNCATE of several relations with identity and drop behaviour",
            sql: "TRUNCATE TABLE ONLY a.b, c * RESTART IDENTITY CASCADE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CONTINUE without IDENTITY in TRUNCATE",
            sql: "TRUNCATE t CONTINUE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
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

#[test]
fn given_sql_when_parsing_then_the_tree_follows_postgresqls_grammar() {
    let test_cases = [
        ShapeTestCase {
            description: "AND binds tighter than OR",
            sql: "SELECT a OR b AND c",
            pointer: "/0/select/expressions/0/or/right",
            expected_node: "and",
        },
        ShapeTestCase {
            description: "NOT binds looser than a comparison",
            sql: "SELECT NOT a = b",
            pointer: "/0/select/expressions/0/not/this",
            expected_node: "eq",
        },
        ShapeTestCase {
            description: "a cast binds tighter than unary minus",
            sql: "SELECT -x::int",
            pointer: "/0/select/expressions/0/neg/this",
            expected_node: "cast",
        },
        ShapeTestCase {
            description: "IS NULL applies to a whole comparison",
            sql: "SELECT a = b IS NULL",
            pointer: "/0/select/expressions/0/is_null/this",
            expected_node: "eq",
        },
        ShapeTestCase {
            description: "a keyword after an expression is its bare label",
            sql: "SELECT 1 like FROM t",
            pointer: "/0/select/expressions/0",
            expected_node: "alias",
        },
        ShapeTestCase {
            description: "a type name before a string is a typed literal",
            sql: "SELECT int4 '1'",
            pointer: "/0/select/expressions/0",
            expected_node: "cast",
        },
        ShapeTestCase {
            description: "the qualifier after a nested join belongs to the inner join",
            sql: "SELECT * FROM a JOIN b JOIN c ON true ON true",
            pointer: "/0/select/joins/0/this",
            expected_node: "joined_table",
        },
        ShapeTestCase {
            description: "INTERSECT binds tighter than UNION",
            sql: "SELECT 1 UNION SELECT 2 INTERSECT SELECT 3",
            pointer: "/0/union/right",
            expected_node: "intersect",
        },
        ShapeTestCase {
            description: "ORDER BY after a set operation belongs to the whole operation",
            sql: "SELECT 1 UNION SELECT 2 ORDER BY 1",
            pointer: "/0/union/order_by",
            expected_node: "expressions",
        },
        ShapeTestCase {
            description: "ON CONFLICT is kept on the insert",
            sql: "INSERT INTO t VALUES (1) ON CONFLICT DO NOTHING",
            pointer: "/0/insert/on_conflict",
            expected_node: "on_conflict",
        },
        ShapeTestCase {
            description: "COPY with a column list names the table and its columns",
            sql: "COPY t (a, b) FROM STDIN",
            pointer: "/0/copy/this",
            expected_node: "schema",
        },
        ShapeTestCase {
            description: "TRUNCATE keeps every relation after the first",
            sql: "TRUNCATE a, b",
            pointer: "/0/truncate/extra_tables/0",
            expected_node: "table",
        },
    ];
    for test_case in test_cases {
        let parsed = parse(test_case.sql).expect(test_case.description);
        let tree: Value = serde_json::to_value(&parsed.statements).expect(test_case.description);
        let node: Option<&Value> = tree.pointer(test_case.pointer);
        assert!(
            node.and_then(Value::as_object)
                .is_some_and(|object| object.contains_key(test_case.expected_node)),
            "{}: {node:?}",
            test_case.description
        );
    }
}
