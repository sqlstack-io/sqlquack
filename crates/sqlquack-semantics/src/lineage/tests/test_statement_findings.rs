use sqlquack_syntax::{CallMode, Dialect, ParseOptions, parse_statements_with};

use crate::dialects::constants::{DUCKDB, POSTGRES, SNOWFLAKE};
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::helpers::{
    duckdb_parser_findings, duckdb_shop_schema, found_findings, owned_findings, shop_schema,
    typed_duckdb_shop_schema,
};
use crate::lineage::tests::test_types::QueryFindingsTestCase;

#[test]
fn given_a_query_when_resolving_its_names_then_unknown_ambiguous_and_unqualifiable_ones_are_found()
{
    let test_cases = [
        QueryFindingsTestCase {
            description: "a qualified column its relation lacks",
            sql: "SELECT o.qty FROM orders o",
            expected_findings: &[("E201", "Unknown column 'qty' in table 'orders'", 9)],
        },
        QueryFindingsTestCase {
            description: "an unqualified column the one relation lacks",
            sql: "SELECT qty FROM orders",
            expected_findings: &[("E201", "Unknown column 'qty' in table 'orders'", 7)],
        },
        QueryFindingsTestCase {
            description: "an unqualified column no relation has",
            sql: "SELECT qty FROM orders, customers",
            expected_findings: &[(
                "E201",
                "Unknown column 'qty' (not found in any referenced table)",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "an unqualified column two relations have",
            sql: "SELECT id FROM orders JOIN customers ON TRUE",
            expected_findings: &[(
                "E221",
                "Ambiguous unqualified column 'id' found in 2 referenced tables",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a USING key is not ambiguous",
            sql: "SELECT id FROM orders JOIN customers USING (id)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a qualifier naming no FROM item",
            sql: "SELECT x.id FROM orders o",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'x' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a correlated reference resolves in the enclosing level",
            sql: "SELECT id FROM orders o WHERE EXISTS (SELECT 1 FROM customers c WHERE c.id = o.customer_id AND c.nam = 'x')",
            expected_findings: &[("E201", "Unknown column 'nam' in table 'customers'", 97)],
        },
        QueryFindingsTestCase {
            description: "CTE columns, open relations, lambdas and output aliases in ORDER BY",
            sql: "WITH c AS (SELECT id AS key FROM orders) \
                  SELECT c.key, u.anything, TRANSFORM(c.key, x -> x + 1) AS t FROM c, unknown_rel u ORDER BY t",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a CTE body reads the relation it shadows; outer findings come first",
            sql: "WITH orders AS (SELECT idd FROM orders) SELECT o.x FROM orders o",
            expected_findings: &[
                ("E201", "Unknown column 'x' in table 'orders'", 49),
                ("E201", "Unknown column 'idd' in table 'orders'", 23),
            ],
        },
        QueryFindingsTestCase {
            description: "lateral items, date parts and partial column aliases",
            sql: "SELECT g.item, DATEDIFF(day, o.id, o.id) AS d, r.amount \
                  FROM orders o, LATERAL (SELECT o.status AS item) g, \
                  (SELECT id, amount FROM orders) AS r(first_id)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a USING key one side lacks, and an UNPIVOT operand its input lacks",
            sql: "SELECT * FROM orders JOIN customers USING (status) \
                  UNION ALL SELECT * FROM orders UNPIVOT (v FOR k IN (missing))",
            expected_findings: &[
                (
                    "E201",
                    "JOIN USING column 'status' must exist on both sides",
                    0,
                ),
                ("E201", "Unknown UNPIVOT input column 'missing'", 103),
            ],
        },
        QueryFindingsTestCase {
            description: "a projection does not read its own alias; ungrouped amount comes first",
            sql: "SELECT MAX(qty) AS qty, amount + total AS total, total FROM orders",
            expected_findings: &[
                (
                    "E230",
                    "Column must be grouped or used in an aggregate function",
                    24,
                ),
                ("E201", "Unknown column 'qty' in table 'orders'", 11),
                ("E201", "Unknown column 'total' in table 'orders'", 33),
                ("E201", "Unknown column 'total' in table 'orders'", 49),
            ],
        },
        QueryFindingsTestCase {
            description: "a lateral star over an earlier lateral item offers its columns",
            sql: "SELECT ss2.status FROM orders o, LATERAL (SELECT o.status) AS ss1, \
                  LATERAL (SELECT ss1.* FROM customers LIMIT 1) AS ss2",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a lateral star over an earlier item still checks what it offers",
            sql: "SELECT ss2.missing FROM orders o, LATERAL (SELECT o.status) AS ss1, \
                  LATERAL (SELECT ss1.* FROM customers LIMIT 1) AS ss2",
            expected_findings: &[("E201", "Unknown column 'missing' in table 'ss2'", 11)],
        },
        QueryFindingsTestCase {
            description: "a CTE column the CTE does not output",
            sql: "WITH c AS (SELECT id FROM orders) SELECT c.amount FROM c",
            expected_findings: &[("E201", "Unknown column 'amount' in table 'c'", 43)],
        },
    ];
    let schema = shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: &SNOWFLAKE,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let statements = parse_statements_with(
            test_case.sql,
            Dialect::Snowflake,
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
fn given_a_snowflake_name_not_definitely_unresolved_when_resolving_names_then_nothing_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "stream METADATA$ columns",
            sql: "SELECT id, METADATA$ACTION, METADATA$ISUPDATE, o.METADATA$ROW_ID FROM orders o",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "CHANGES metadata columns",
            sql: "SELECT id, METADATA$ACTION FROM orders CHANGES (INFORMATION => DEFAULT) AT (OFFSET => -60)",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a schema-qualified column",
            sql: "SELECT shop.products.sku FROM shop.products",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "sequence values",
            sql: "SELECT my_seq.NEXTVAL, shop.my_seq.NEXTVAL, my_seq.currval, id FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a MATCH_RECOGNIZE alias",
            sql: "SELECT m.customer_id, m.first_amount FROM orders MATCH_RECOGNIZE (\
             PARTITION BY customer_id ORDER BY id MEASURES FIRST(amount) AS first_amount \
             ONE ROW PER MATCH PATTERN (a+) DEFINE a AS amount > 0) AS m",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "LAST_DAY's date part is its second argument",
            sql: "SELECT LAST_DAY(o.id, month) AS d, LAST_DAY(o.id, week) AS w FROM orders o",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "positional columns, qualified or not",
            sql: "SELECT o.$2, $1 FROM orders o",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a forward reference to a later alias",
            sql: "SELECT later + 1 AS earlier, id AS later FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a correlated reference to an enclosing output alias",
            sql: "SELECT id AS order_key, (SELECT COUNT(*) FROM customers c WHERE c.id = order_key) AS n \
             FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unaliased expression output read by its generated name",
            sql: "SELECT t.\"AMOUNT + 1\" FROM (SELECT amount + 1 FROM orders) t",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an unaliased FLATTEN read under its own name",
            sql: "SELECT flatten.value FROM orders AS o, LATERAL FLATTEN(INPUT => o.items)",
            expected_findings: &[],
        },
    ];
    let schema = shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::Snowflake, &SNOWFLAKE, &schema)),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

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
fn given_a_postgres_relation_with_a_finding_when_resolving_names_then_only_its_follow_ons_are_dropped()
 {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a name read through a star of a faulted CTE is a follow-on of the root finding",
            sql: "WITH kept AS (SELECT customer_id_missing FROM orders), again AS (SELECT * FROM kept) \
                  SELECT customer_id FROM again",
            expected_findings: &[(
                "E201",
                "Unknown column 'customer_id_missing' in table 'orders'",
                21,
            )],
        },
        QueryFindingsTestCase {
            description: "the root finding and a finding independent of the faulted CTE both appear",
            sql: "WITH kept AS (SELECT customer_id_missing FROM orders), again AS (SELECT * FROM kept) \
                  SELECT a.customer_id, c.nam FROM again a, customers c",
            expected_findings: &[
                ("E201", "Unknown column 'nam' in table 'customers'", 109),
                (
                    "E201",
                    "Unknown column 'customer_id_missing' in table 'orders'",
                    21,
                ),
            ],
        },
        QueryFindingsTestCase {
            description: "a name read from a derived table with a finding is a follow-on of the root finding",
            sql: "SELECT d.customer_id FROM (SELECT customer_id_missing FROM orders) d",
            expected_findings: &[(
                "E201",
                "Unknown column 'customer_id_missing' in table 'orders'",
                34,
            )],
        },
        QueryFindingsTestCase {
            description: "a derived table reading a faulted CTE is faulted; an independent error still appears",
            sql: "WITH kept AS (SELECT customer_id_missing FROM orders) \
                  SELECT d.customer_id, c.nam FROM (SELECT * FROM kept) d, customers c",
            expected_findings: &[
                ("E201", "Unknown column 'nam' in table 'customers'", 78),
                (
                    "E201",
                    "Unknown column 'customer_id_missing' in table 'orders'",
                    21,
                ),
            ],
        },
        QueryFindingsTestCase {
            description: "a derived table without a finding still has its unknown columns found",
            sql: "SELECT d.nope FROM (SELECT customer_id FROM orders) d",
            expected_findings: &[("E201", "Unknown column 'nope' in table 'd'", 9)],
        },
        QueryFindingsTestCase {
            description: "a CTE without a finding still has its unknown columns found",
            sql: "WITH kept AS (SELECT customer_id FROM orders) SELECT k.nope FROM kept k",
            expected_findings: &[("E201", "Unknown column 'nope' in table 'kept'", 55)],
        },
    ];
    let schema = duckdb_shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::PostgreSQL, &POSTGRES, &schema)),
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
fn given_a_snowflake_query_when_a_name_or_window_is_definitely_wrong_then_it_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a qualifier over a VALUES list",
            sql: "SELECT zq.d FROM VALUES ('x') AS v(d)",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'd'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a qualifier beside an unaliased FLATTEN",
            sql: "SELECT zq.value FROM orders AS o, LATERAL FLATTEN(INPUT => o.items)",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'value'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a qualifier over a relation whose columns are unknown",
            sql: "SELECT zq.id FROM undeclared AS u",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "ranking calls without OVER, beside one with it",
            sql: "SELECT id, ROW_NUMBER() AS n, LAG(id) AS l, RANK() OVER (ORDER BY id) AS r FROM orders",
            expected_findings: &[
                ("E232", "Window function requires an OVER clause", 11),
                ("E232", "Window function requires an OVER clause", 30),
            ],
        },
    ];
    let schema = shop_schema();
    for test_case in test_cases {
        assert_eq!(
            found_findings(test_case.sql, (Dialect::Snowflake, &SNOWFLAKE, &schema)),
            owned_findings(test_case.expected_findings),
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
