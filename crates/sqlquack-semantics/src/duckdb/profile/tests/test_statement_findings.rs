use crate::lineage::tests::helpers::typed_duckdb_shop_schema;

use sqlquack_parse::parse_statements_with;
use sqlquack_syntax::{CallMode, Dialect, ParseOptions};

use crate::dialects::constants::DUCKDB;
use crate::duckdb::profile::tests::helpers::duckdb_parser_findings;
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::helpers::{
    duckdb_shop_schema, found_findings, owned_findings, shop_schema,
};
use crate::lineage::tests::test_types::QueryFindingsTestCase;

#[test]
fn given_a_duckdb_name_not_definitely_unresolved_when_resolving_names_then_nothing_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "an unaliased expression output read by its generated name on DuckDB",
            sql: "SELECT \"(amount + 1)\" FROM (SELECT amount + 1 FROM orders) t",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a SEMI join's right side does not make a name ambiguous",
            sql: "SELECT id FROM orders o SEMI JOIN customers c ON o.customer_id = c.id",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an ANTI join's right side does not make a name ambiguous",
            sql: "SELECT id FROM orders o ANTI JOIN customers c ON o.customer_id = c.id",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a set operation's ORDER BY naming an output in upper case",
            sql: "SELECT id FROM orders UNION ALL SELECT id FROM customers ORDER BY ID",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a set operation's ORDER BY naming an upper-case output in lower case",
            sql: "SELECT ID FROM orders UNION ALL SELECT ID FROM customers ORDER BY id",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a list comprehension's variable",
            sql: "SELECT [x * 2 FOR x IN items IF x > 0] AS doubled FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "DuckDB's rowid",
            sql: "SELECT rowid, id FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a struct field read through its column",
            sql: "SELECT items.city, o.items.city FROM orders o",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a struct field of a derived column",
            sql: "SELECT payload.city FROM (SELECT {'city': 'x'} AS payload) t",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unaliased table function read under its own name",
            sql: "SELECT range.range FROM range(3)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unaliased open table function read under its own name",
            sql: "SELECT read_parquet.id FROM read_parquet('orders.parquet')",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unaliased file scan read under its file's stem",
            sql: "SELECT y.id FROM 'exports/y.csv'",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a struct field of a lambda parameter over an open relation",
            sql: "SELECT list_transform(u.items, lambda x: x.city) AS cities FROM undeclared u",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a struct field of a list comprehension's variable",
            sql: "SELECT [x.city FOR x IN u.items] AS cities FROM undeclared u",
            expected_findings: &[],
        },
    ];
    let schema = duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::DuckDB, &DUCKDB, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_duckdb_query_when_a_name_is_definitely_unresolved_then_it_is_still_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "an unknown column of a closed relation",
            sql: "SELECT qty FROM orders",
            expected_findings: &[("E201", "Unknown column 'qty' in table 'orders'", 7)],
        },
        QueryFindingsTestCase {
            description: "a set operation's ORDER BY naming a branch column that is not an output",
            sql: "SELECT id FROM orders UNION ALL SELECT id FROM customers ORDER BY customer_id",
            expected_findings: &[(
                "E201",
                "Set-operation ORDER BY references a non-output column",
                66,
            )],
        },
        QueryFindingsTestCase {
            description: "a comprehension binds its variable, not its iterator",
            sql: "SELECT [x * 2 FOR x IN missing_items] AS doubled FROM orders",
            expected_findings: &[(
                "E201",
                "Unknown column 'missing_items' in table 'orders'",
                23,
            )],
        },
        QueryFindingsTestCase {
            description: "an inner join keeps both sides in the ambiguity count",
            sql: "SELECT id FROM orders o JOIN customers c ON o.customer_id = c.id",
            expected_findings: &[(
                "E221",
                "Ambiguous unqualified column 'id' found in 2 referenced tables",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a qualifier that is neither an alias nor a column",
            sql: "SELECT x.id FROM orders o",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'x' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a qualifier beside a source whose columns are unknown",
            sql: "SELECT zq.id FROM orders o JOIN undeclared u ON TRUE",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a qualifier over a renamed VALUES list",
            sql: "SELECT zq.d FROM (VALUES (1)) AS v(d)",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'd'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a file scan's stem once the scan is aliased",
            sql: "SELECT y.id FROM 'exports/y.csv' AS q",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'y' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a qualifier other than an unaliased table function's own name",
            sql: "SELECT zq.range FROM range(3)",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'range'",
                7,
            )],
        },
    ];
    let schema = duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::DuckDB, &DUCKDB, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_duckdb_lateral_star_over_an_earlier_item_when_resolving_names_then_its_columns_resolve()
{
    let test_cases = [
        QueryFindingsTestCase {
            description: "a DuckDB lateral star over an earlier lateral item offers its columns",
            sql: "SELECT ss2.status FROM orders o, LATERAL (SELECT o.status) AS ss1, \
                  LATERAL (SELECT ss1.* FROM customers LIMIT 1) AS ss2 WHERE ss2.status = 'open'",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a DuckDB lateral star still checks the columns it offers",
            sql: "SELECT ss2.missing FROM orders o, LATERAL (SELECT o.status) AS ss1, \
                  LATERAL (SELECT ss1.* FROM customers LIMIT 1) AS ss2",
            expected_findings: &[("E201", "Unknown column 'missing' in table 'ss2'", 11)],
        },
    ];
    let schema = shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: &DUCKDB,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let statements = parse_statements_with(
            test_case.sql,
            Dialect::DuckDB,
            &ParseOptions::default(),
            CallMode::FaithfulLocated,
        )
        .unwrap_or_else(|error| panic!("{}: {error}", test_case.description));
        let found: Vec<(&str, String, usize)> = statement_findings(&statements[0], context)
            .into_iter()
            .map(|finding| {
                (
                    finding.code,
                    finding.message,
                    finding.span.map_or(0, |span| span.start),
                )
            })
            .collect();
        assert_eq!(
            found,
            test_case
                .expected_findings
                .iter()
                .map(|(code, message, start)| (*code, (*message).to_owned(), *start))
                .collect::<Vec<(&str, String, usize)>>(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_bare_duckdb_value_keyword_from_duckdbs_parser_when_resolving_names_then_only_columns_are_judged()
 {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a keyword no FROM item offers reads as its function",
            sql: "WITH recent AS (SELECT CURRENT_TIMESTAMP AS created_at) \
                  SELECT created_at FROM recent WHERE created_at > CURRENT_TIMESTAMP - INTERVAL 1 DAY",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "every value keyword DuckDB reads bare, beside a relation",
            sql: "SELECT current_catalog, current_date, current_role, current_schema, \
                  current_time, current_timestamp, current_user, localtime, localtimestamp, \
                  session_user, user FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a keyword in an aggregate query, neither grouped nor aggregated",
            sql: "SELECT current_date, count(*) AS n FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a keyword beside a GROUP BY key",
            sql: "SELECT id, current_date FROM orders GROUP BY id",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a keyword two CTEs offer as a column is ambiguous",
            sql: "WITH a AS (SELECT 1 AS current_date), b AS (SELECT 2 AS current_date) \
                  SELECT current_date FROM a, b",
            expected_findings: &[(
                "E221",
                "Ambiguous unqualified column 'current_date' found in 2 referenced tables",
                77,
            )],
        },
        QueryFindingsTestCase {
            description: "a keyword a CTE offers as a column is judged for grouping",
            sql: "WITH a AS (SELECT 1 AS current_date, 2 AS id) \
                  SELECT current_date, count(*) AS n FROM a",
            expected_findings: &[(
                "E230",
                "Column must be grouped or used in an aggregate function",
                53,
            )],
        },
        QueryFindingsTestCase {
            description: "a function name DuckDB does not read bare is still a column",
            sql: "SELECT now FROM orders",
            expected_findings: &[("E201", "Unknown column 'now' in table 'orders'", 7)],
        },
        QueryFindingsTestCase {
            description: "a qualified keyword is a column of its relation",
            sql: "SELECT o.current_date FROM orders o",
            expected_findings: &[("E201", "Unknown column 'current_date' in table 'orders'", 9)],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            duckdb_parser_findings(test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_aggregate_filter_from_duckdbs_parser_when_checking_types_then_a_text_condition_is_found()
 {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a text FILTER condition on COUNT(*)",
            sql: "SELECT COUNT(*) FILTER (WHERE status) AS n FROM orders",
            expected_findings: &[("W215", "Expected boolean condition, found string", 30)],
        },
        QueryFindingsTestCase {
            description: "a comparison FILTER condition",
            sql: "SELECT COUNT(*) FILTER (WHERE status = 'open') AS n FROM orders",
            expected_findings: &[],
        },
    ];
    let schema = typed_duckdb_shop_schema();
    for test_case in test_cases {
        let statement = sqlquack_syntax_duckdb::parse_strict(test_case.sql)
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", test_case.description))
            .statements
            .remove(0);
        let context = LineageContext {
            profile: &DUCKDB,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let findings: Vec<(&'static str, String, usize)> = statement_findings(&statement, context)
            .into_iter()
            .map(|finding| {
                (
                    finding.code,
                    finding.message,
                    finding.span.map_or(0, |span| span.start),
                )
            })
            .collect();
        assert_eq!(
            findings,
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_duckdb_syntax_its_parser_emits_as_calls_when_checking_then_the_forms_duckdb_runs_are_known()
 {
    let test_cases = [
        QueryFindingsTestCase {
            description: "EXTRACT, which DuckDB rewrites to date_part",
            sql: "SELECT EXTRACT(YEAR FROM amount) AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "date_part, the call EXTRACT becomes",
            sql: "SELECT date_part('year', amount) AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an ARRAY subquery",
            sql: "SELECT ARRAY(SELECT id FROM orders) AS v",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a postfix factorial",
            sql: "SELECT amount! AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a prefix operator DuckDB has",
            sql: "SELECT @amount AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a prefix OPERATOR() form DuckDB has",
            sql: "SELECT OPERATOR(-) amount AS v FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "GROUP BY GROUPING SETS",
            sql: "SELECT status, id, count(*) AS n FROM orders GROUP BY GROUPING SETS ((status), (id))",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a prefix operator DuckDB does not have stays unknown",
            sql: "SELECT |/ amount AS v FROM orders",
            expected_findings: &[("E202", "Unknown function '|/' for dialect DuckDB", 0)],
        },
        QueryFindingsTestCase {
            description: "a name DuckDB has nothing for stays unknown",
            sql: "SELECT not_a_function(amount) AS v FROM orders",
            expected_findings: &[(
                "E202",
                "Unknown function 'not_a_function' for dialect DuckDB",
                7,
            )],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            duckdb_parser_findings(test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_derived_table_inside_a_lateral_subquery_when_resolving_names_then_it_reads_the_enclosing_items()
 {
    // Each verdict is DuckDB 2.0.0's: the derived tables reading p run, the one reading q does not.
    let test_cases = [
        QueryFindingsTestCase {
            description: "a comma-joined LATERAL whose nested derived table reads the item before it",
            sql: "SELECT k.n FROM orders AS p, \
                  LATERAL (SELECT f.u AS n FROM (SELECT UNNEST(p.items) AS u) AS f) AS k",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "the same body under CROSS JOIN LATERAL",
            sql: "SELECT k.n FROM orders AS p \
                  CROSS JOIN LATERAL (SELECT f.u AS n FROM (SELECT UNNEST(p.items) AS u) AS f) AS k",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a scalar subquery whose derived table reads the outer query",
            sql: "SELECT (SELECT f.u FROM (SELECT p.items AS u) AS f) AS n FROM orders AS p",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a nested derived table still reports an alias no enclosing query has",
            sql: "SELECT k.n FROM orders AS p, \
                  LATERAL (SELECT f.u AS n FROM (SELECT q.items AS u) AS f) AS k",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'q' referenced by column 'items'",
                67,
            )],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            duckdb_parser_findings(test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_duckdbs_parser_reads_window_calls_when_checking_arity_then_every_count_duckdb_binds_is_known()
 {
    // Each verdict is DuckDB 2.0.0's: LAG and LEAD bind one to three arguments, not four.
    let test_cases = [
        QueryFindingsTestCase {
            description: "LAG and LEAD with one, two and three arguments",
            sql: "SELECT LAG(amount) OVER (ORDER BY id) AS a, LAG(amount, 2) OVER (ORDER BY id) AS b, \
                  LAG(amount, 1, 0) OVER (ORDER BY id) AS c, LEAD(amount) OVER (ORDER BY id) AS d, \
                  LEAD(amount, 1) OVER (ORDER BY id) AS e FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a short LEAD inside QUALIFY, COALESCE and CASE",
            sql: "SELECT COALESCE(LEAD(amount) OVER (ORDER BY id), 0) AS a, \
                  CASE WHEN LAG(status) OVER (ORDER BY id) IS NULL THEN 1 END AS b \
                  FROM orders QUALIFY LAG(amount) OVER (ORDER BY id) > 1",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "the other window functions at the counts DuckDB binds",
            sql: "SELECT NTH_VALUE(amount, 2) OVER (ORDER BY id) AS a, NTILE(4) OVER (ORDER BY id) AS b, \
                  FIRST_VALUE(amount) OVER (ORDER BY id) AS c, LAST_VALUE(amount) OVER (ORDER BY id) AS d, \
                  PERCENT_RANK() OVER (ORDER BY id) AS e, CUME_DIST() OVER (ORDER BY id) AS f, \
                  ROW_NUMBER() OVER () AS g, RANK() OVER (ORDER BY id) AS h, \
                  DENSE_RANK() OVER (ORDER BY id) AS i FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "LAG with four arguments is still refused",
            sql: "SELECT LAG(amount, 1, 0, 2) OVER (ORDER BY id) AS a FROM orders",
            expected_findings: &[(
                "E203",
                "Invalid arity for function 'LAG': got 4, expected 1, 2, 3",
                11,
            )],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            duckdb_parser_findings(test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}
