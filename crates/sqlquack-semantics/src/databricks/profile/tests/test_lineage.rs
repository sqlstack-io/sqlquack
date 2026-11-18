use crate::databricks::profile::tests::helpers::databricks_findings;
use crate::databricks::profile::tests::helpers::databricks_outputs;
use crate::databricks::profile::tests::helpers::reading;
use crate::databricks::profile::tests::test_types::DatabricksOutputsTestCase;
use crate::databricks::profile::tests::test_types::SameReadingTestCase;
use crate::dialects::constants::DATABRICKS;
use crate::lineage::tests::helpers::duckdb_shop_schema;
use crate::lineage::tests::helpers::output_ranges;
use crate::lineage::tests::helpers::owned_findings;
use crate::lineage::tests::test_types::QueryFindingsTestCase;
use crate::lineage::tests::test_types::StarOrderTestCase;
use crate::lineage::tests::test_types::StrictRangeTestCase;

#[test]
fn given_a_databricks_query_when_checking_it_then_only_rules_databricks_shares_are_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "DATE_TRUNC over a STRING column, which Snowflake's rules refuse",
            sql: "SELECT DATE_TRUNC('day', status) AS d FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a STRING WHERE condition and a STRING compared with a number",
            sql: "SELECT id FROM orders WHERE status AND status > 1",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "two FROM items under one alias, which Databricks accepts",
            sql: "SELECT 1 AS one FROM orders o JOIN customers o ON TRUE",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a lateral column alias read by a later output",
            sql: "SELECT quantity AS q, q + 1 AS r FROM orders",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a LATERAL VIEW's generated column",
            sql: "SELECT x FROM orders LATERAL VIEW explode(array(1, 2)) t AS x",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a LATERAL VIEW OUTER's generated column",
            sql: "SELECT x FROM orders LATERAL VIEW OUTER explode(array(1, 2)) t AS x",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a LATERAL VIEW over a map, with a key and a value column",
            sql: "SELECT k, v FROM orders LATERAL VIEW explode(map(1, 2)) t AS k, v",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a generated column qualified by the LATERAL VIEW's alias",
            sql: "SELECT t.x FROM orders LATERAL VIEW explode(array(1, 2)) t AS x",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a LATERAL VIEW whose generator reads the FROM relation",
            sql: "SELECT o.id, x FROM orders o LATERAL VIEW explode(array(o.quantity)) t AS x",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a LATERAL VIEW without column aliases, read by the generator's own name",
            sql: "SELECT col FROM orders LATERAL VIEW explode(array(1, 2)) t",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a column neither the relation nor the LATERAL VIEW offers",
            sql: "SELECT y FROM orders LATERAL VIEW explode(array(1, 2)) t AS x",
            expected_findings: &[(
                "E201",
                "Unknown column 'y' (not found in any referenced table)",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a qualifier that is neither the relation nor the LATERAL VIEW",
            sql: "SELECT u.x FROM orders LATERAL VIEW explode(array(1, 2)) t AS x",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'u' referenced by column 'x'",
                7,
            )],
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
            description: "a set operation's ORDER BY naming a branch column that is not an output",
            sql: "SELECT id FROM orders UNION ALL SELECT id FROM customers ORDER BY quantity",
            expected_findings: &[(
                "E201",
                "Set-operation ORDER BY references a non-output column",
                66,
            )],
        },
        QueryFindingsTestCase {
            description: "two CTEs under one name (DUPLICATED_CTE_NAMES)",
            sql: "WITH c AS (SELECT 1 AS x), c AS (SELECT 2 AS x) SELECT x FROM c",
            expected_findings: &[("E233", "Duplicate CTE name 'c'", 55)],
        },
        QueryFindingsTestCase {
            description: "QUALIFY without a window function (QUALIFY must be used with one)",
            sql: "SELECT id FROM orders QUALIFY id > 0",
            expected_findings: &[(
                "E232",
                "QUALIFY requires a window function in the same query scope",
                7,
            )],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            databricks_findings(test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_databricks_query_when_naming_and_typing_outputs_then_they_are_the_warehouses_or_none() {
    let test_cases = [
        DatabricksOutputsTestCase {
            description: "a LATERAL VIEW's generated column is untyped beside a typed column",
            sql: "SELECT id, x FROM orders LATERAL VIEW explode(array(quantity)) t AS x",
            expected_outputs: &[("id", Some("INT")), ("x", None)],
        },
        DatabricksOutputsTestCase {
            description: "literals typed as typeof reports them",
            sql: "SELECT 1 AS a, 2147483648 AS b, 9223372036854775808 AS c, 1.50 AS d, \
                  0.05 AS e, 1e5 AS f, 'x' AS g, -2147483648 AS h, r'x' AS i",
            expected_outputs: &[
                ("a", Some("INT")),
                ("b", Some("BIGINT")),
                ("c", Some("DECIMAL(19,0)")),
                ("d", Some("DECIMAL(3,2)")),
                ("e", Some("DECIMAL(2,2)")),
                ("f", Some("DOUBLE")),
                ("g", Some("STRING")),
                ("h", Some("INT")),
                ("i", Some("STRING")),
            ],
        },
        DatabricksOutputsTestCase {
            description: "casts typed as DESCRIBE QUERY reports them",
            sql: "SELECT CAST(1 AS TIMESTAMP) AS i, CAST('2020-01-01' AS TIMESTAMP_NTZ) AS j, \
                  CAST(1 AS DECIMAL) AS k, CAST('a' AS VARCHAR(3)) AS l",
            expected_outputs: &[
                ("i", Some("TIMESTAMP")),
                ("j", Some("TIMESTAMP_NTZ")),
                ("k", Some("DECIMAL(10,0)")),
                ("l", Some("STRING")),
            ],
        },
        DatabricksOutputsTestCase {
            description: "operators and lateral aliases, untyped where Databricks is not recorded",
            sql: "SELECT status || status AS m, quantity + quantity AS n, \
                  quantity / quantity AS o, ROUND(price, 1) AS p, quantity AS q, q * 2 AS r \
                  FROM orders",
            expected_outputs: &[
                ("m", Some("STRING")),
                ("n", Some("INT")),
                ("o", None),
                ("p", None),
                ("q", Some("INT")),
                ("r", Some("INT")),
            ],
        },
        DatabricksOutputsTestCase {
            description: "VALUES columns named col1, col2",
            sql: "SELECT * FROM VALUES (1, 'a')",
            expected_outputs: &[("col1", Some("INT")), ("col2", Some("STRING"))],
        },
        DatabricksOutputsTestCase {
            description: "an unaliased expression left unnamed, not given Snowflake's name",
            sql: "SELECT status, UPPER(status) FROM orders",
            expected_outputs: &[("status", Some("STRING")), ("", None)],
        },
    ];
    for test_case in test_cases {
        let found: Vec<(String, Option<String>)> = databricks_outputs(test_case.sql);
        let found: Vec<(&str, Option<&str>)> = found
            .iter()
            .map(|(name, data_type)| (name.as_str(), data_type.as_deref()))
            .collect();
        assert_eq!(
            found, test_case.expected_outputs,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_strict_parse_when_tracing_lineage_then_each_output_has_its_items_byte_range() {
    let test_cases = [
        StrictRangeTestCase {
            description: "a literal aliased after a two-byte character, a qualified column, a labelled call, a bare column and a qualified star",
            sql: "SELECT 'é' AS e, w.status, UPPER(w.status) loud, id, w.* FROM orders AS w",
            expected_ranges: &[
                7..16,
                18..26,
                28..48,
                50..52,
                54..57,
                54..57,
                54..57,
                54..57,
                54..57,
            ],
        },
        StrictRangeTestCase {
            description: "a pipe ORDER BY after a parenthesized query",
            sql: "( SELECT id, quantity FROM orders ) |> ORDER BY status",
            expected_ranges: &[9..11, 13..21],
        },
        StrictRangeTestCase {
            description: "a pipe LIMIT after a parenthesized query",
            sql: "( SELECT id, quantity FROM orders ) |> LIMIT 1",
            expected_ranges: &[9..11, 13..21],
        },
    ];
    for test_case in test_cases {
        let statement = sqlquack_syntax_databricks::parse_strict(test_case.sql)
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", test_case.description))
            .statements
            .remove(0);
        assert_eq!(
            output_ranges(
                &statement,
                &DATABRICKS,
                &duckdb_shop_schema(),
                test_case.sql
            ),
            test_case
                .expected_ranges
                .iter()
                .cloned()
                .map(Some)
                .collect::<Vec<_>>(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_aggregate_written_with_all_when_analysed_then_it_reads_as_no_quantifier() {
    let test_cases = [
        SameReadingTestCase {
            description: "SUM with ALL",
            sql: "SELECT sum(ALL quantity) AS a FROM orders HAVING sum(ALL quantity) > 0",
            expected_same_as: "SELECT sum(quantity) AS a FROM orders HAVING sum(quantity) > 0",
        },
        SameReadingTestCase {
            description: "AVG with ALL",
            sql: "SELECT avg(ALL price) AS a FROM orders HAVING avg(ALL price) > 0",
            expected_same_as: "SELECT avg(price) AS a FROM orders HAVING avg(price) > 0",
        },
        SameReadingTestCase {
            description: "COUNT with ALL",
            sql: "SELECT count(ALL status) AS a FROM orders HAVING count(ALL status) > 0",
            expected_same_as: "SELECT count(status) AS a FROM orders HAVING count(status) > 0",
        },
        SameReadingTestCase {
            description: "MAX with ALL",
            sql: "SELECT max(ALL status) AS a FROM orders HAVING max(ALL status) > 0",
            expected_same_as: "SELECT max(status) AS a FROM orders HAVING max(status) > 0",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            (
                databricks_outputs(test_case.sql),
                databricks_findings(test_case.sql)
            ),
            (
                databricks_outputs(test_case.expected_same_as),
                databricks_findings(test_case.expected_same_as)
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_synonymous_operator_spellings_when_analysed_then_each_reads_alike() {
    let test_cases = [
        SameReadingTestCase {
            description: "== against =",
            sql: "SELECT quantity == 2 AS a FROM orders WHERE quantity == 2",
            expected_same_as: "SELECT quantity = 2 AS a FROM orders WHERE quantity = 2",
        },
        SameReadingTestCase {
            description: "== against = between a string and a number",
            sql: "SELECT status == price AS a FROM orders WHERE status == price",
            expected_same_as: "SELECT status = price AS a FROM orders WHERE status = price",
        },
        SameReadingTestCase {
            description: "!= against <>",
            sql: "SELECT quantity != 2 AS a FROM orders WHERE quantity != 2",
            expected_same_as: "SELECT quantity <> 2 AS a FROM orders WHERE quantity <> 2",
        },
        SameReadingTestCase {
            description: "<=> against IS NOT DISTINCT FROM",
            sql: "SELECT quantity <=> 2 AS a FROM orders WHERE quantity <=> 2",
            expected_same_as: "SELECT quantity IS NOT DISTINCT FROM 2 AS a FROM orders WHERE quantity IS NOT DISTINCT FROM 2",
        },
        SameReadingTestCase {
            description: "NOT <=> against IS DISTINCT FROM",
            sql: "SELECT NOT quantity <=> 2 AS a FROM orders WHERE NOT quantity <=> 2",
            expected_same_as: "SELECT quantity IS DISTINCT FROM 2 AS a FROM orders WHERE quantity IS DISTINCT FROM 2",
        },
        SameReadingTestCase {
            description: "REGEXP against RLIKE",
            sql: "SELECT status REGEXP 'x' AS a FROM orders WHERE status REGEXP 'x'",
            expected_same_as: "SELECT status RLIKE 'x' AS a FROM orders WHERE status RLIKE 'x'",
        },
        SameReadingTestCase {
            description: "RLIKE against the rlike call",
            sql: "SELECT status RLIKE 'x' AS a FROM orders WHERE status RLIKE 'x'",
            expected_same_as: "SELECT rlike(status, 'x') AS a FROM orders WHERE rlike(status, 'x')",
        },
        SameReadingTestCase {
            description: ">>> against the shiftrightunsigned call",
            sql: "SELECT quantity >>> 1 AS a FROM orders WHERE quantity >>> 1",
            expected_same_as: "SELECT shiftrightunsigned(quantity, 1) AS a FROM orders WHERE shiftrightunsigned(quantity, 1)",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            reading(test_case.sql),
            reading(test_case.expected_same_as),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_relation_or_query_in_extra_parentheses_when_analysed_then_it_reads_as_without() {
    let test_cases = [
        SameReadingTestCase {
            description: "a table in two pairs of parentheses",
            sql: "SELECT id, nope FROM ( ( orders ) )",
            expected_same_as: "SELECT id, nope FROM orders",
        },
        SameReadingTestCase {
            description: "an aliased table in two pairs of parentheses",
            sql: "SELECT o.id, o.nope FROM ( ( orders ) ) o",
            expected_same_as: "SELECT o.id, o.nope FROM orders o",
        },
        SameReadingTestCase {
            description: "a table function in parentheses",
            sql: "SELECT r.id FROM ( range ( 3 ) ) r",
            expected_same_as: "SELECT r.id FROM range ( 3 ) r",
        },
        SameReadingTestCase {
            description: "a join in a second pair of parentheses",
            sql: "SELECT o.id, c.name FROM ( ( orders o JOIN customers c ON o.id = c.id ) )",
            expected_same_as: "SELECT o.id, c.name FROM ( orders o JOIN customers c ON o.id = c.id )",
        },
        SameReadingTestCase {
            description: "a derived table's query in a second pair of parentheses",
            sql: "SELECT x FROM ( ( SELECT id AS x FROM orders ) ) d",
            expected_same_as: "SELECT x FROM ( SELECT id AS x FROM orders ) d",
        },
        SameReadingTestCase {
            description: "IN and EXISTS subqueries in a second pair of parentheses",
            sql: "SELECT id FROM orders WHERE id IN ( ( SELECT nope FROM customers ) ) AND EXISTS ( ( SELECT nope FROM customers ) )",
            expected_same_as: "SELECT id FROM orders WHERE id IN ( SELECT nope FROM customers ) AND EXISTS ( SELECT nope FROM customers )",
        },
        SameReadingTestCase {
            description: "a whole query in parentheses",
            sql: "( SELECT id, nope FROM orders )",
            expected_same_as: "SELECT id, nope FROM orders",
        },
        SameReadingTestCase {
            description: "a set operation's operands in parentheses",
            sql: "( SELECT id FROM orders ) UNION ( SELECT id FROM customers )",
            expected_same_as: "SELECT id FROM orders UNION SELECT id FROM customers",
        },
        SameReadingTestCase {
            description: "an insert's query in parentheses",
            sql: "INSERT INTO orders ( SELECT id, nope FROM customers )",
            expected_same_as: "INSERT INTO orders SELECT id, nope FROM customers",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            reading(test_case.sql),
            reading(test_case.expected_same_as),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_join_in_parentheses_when_analysed_then_its_conditions_and_items_are_checked_as_without()
{
    let test_cases = [
        SameReadingTestCase {
            description: "an unknown column in a parenthesised join's ON",
            sql: "SELECT o.id FROM ( orders o JOIN customers c ON o.nope = c.id )",
            expected_same_as: "SELECT o.id FROM orders o JOIN customers c ON o.nope = c.id",
        },
        SameReadingTestCase {
            description: "the same join in a second pair of parentheses",
            sql: "SELECT o.id FROM ( ( orders o JOIN customers c ON o.nope = c.id ) )",
            expected_same_as: "SELECT o.id FROM orders o JOIN customers c ON o.nope = c.id",
        },
        SameReadingTestCase {
            description: "a USING key one side lacks",
            sql: "SELECT o.id FROM ( orders o JOIN customers c USING ( nope ) )",
            expected_same_as: "SELECT o.id FROM orders o JOIN customers c USING ( nope )",
        },
        SameReadingTestCase {
            description: "a derived table inside the parenthesised join",
            sql: "SELECT o.id FROM ( orders o JOIN ( SELECT nope FROM customers ) d ON o.id = d.nope )",
            expected_same_as: "SELECT o.id FROM orders o JOIN ( SELECT nope FROM customers ) d ON o.id = d.nope",
        },
        SameReadingTestCase {
            description: "a parenthesised join joined to an earlier table",
            sql: "SELECT o.id FROM customers c2 JOIN ( orders o JOIN customers c ON o.nope = c.id ) ON c2.id = o.id",
            expected_same_as: "SELECT o.id FROM customers c2 JOIN orders o ON c2.id = o.id JOIN customers c ON o.nope = c.id",
        },
        SameReadingTestCase {
            description: "a LATERAL item reading the table before it inside the parentheses",
            sql: "SELECT l.n FROM ( orders o JOIN LATERAL ( SELECT o.id AS n ) l ON true )",
            expected_same_as: "SELECT l.n FROM orders o JOIN LATERAL ( SELECT o.id AS n ) l ON true",
        },
        SameReadingTestCase {
            description: "USING inside the parentheses, its left side only the tables there",
            sql: "SELECT 1 FROM orders JOIN ( customers JOIN orders y USING ( quantity ) ) ON true",
            expected_same_as: "SELECT 1 FROM customers JOIN orders y USING ( quantity )",
        },
        SameReadingTestCase {
            description: "USING after the parentheses, its right side every table there",
            sql: "SELECT 1 FROM orders JOIN ( customers JOIN orders y ON true ) USING ( quantity )",
            expected_same_as: "SELECT 1 FROM orders JOIN orders y USING ( quantity )",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            reading(test_case.sql),
            reading(test_case.expected_same_as),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_parenthesized_query_with_clauses_after_it_when_analysed_then_they_read_its_sources() {
    let test_cases = [
        SameReadingTestCase {
            description: "a named window written after the parentheses",
            sql: "( SELECT sum ( quantity ) OVER w AS x FROM orders LIMIT 1 ) WINDOW w AS ( ORDER BY quantity )",
            expected_same_as: "SELECT sum ( quantity ) OVER w AS x FROM orders WINDOW w AS ( ORDER BY quantity ) LIMIT 1",
        },
        SameReadingTestCase {
            description: "ORDER BY a column the query does not output",
            sql: "( SELECT id FROM orders ) ORDER BY quantity",
            expected_same_as: "SELECT id FROM orders ORDER BY quantity",
        },
        SameReadingTestCase {
            description: "ORDER BY through the query's table alias",
            sql: "( SELECT id FROM orders o ) ORDER BY o.quantity",
            expected_same_as: "SELECT id FROM orders o ORDER BY o.quantity",
        },
        SameReadingTestCase {
            description: "ORDER BY through the table name, then LIMIT",
            sql: "( SELECT id FROM orders ) ORDER BY orders.quantity LIMIT 1",
            expected_same_as: "SELECT id FROM orders ORDER BY orders.quantity LIMIT 1",
        },
        SameReadingTestCase {
            description: "an unknown ORDER BY column is still found",
            sql: "( SELECT id FROM orders ) ORDER BY nope",
            expected_same_as: "SELECT id FROM orders ORDER BY nope",
        },
        SameReadingTestCase {
            description: "two pairs of parentheses",
            sql: "( ( SELECT id FROM orders ) ) ORDER BY nope, quantity",
            expected_same_as: "SELECT id FROM orders ORDER BY nope, quantity",
        },
        SameReadingTestCase {
            description: "inside an IN subquery",
            sql: "SELECT id FROM orders WHERE id IN ( ( SELECT id FROM customers ) ORDER BY name )",
            expected_same_as: "SELECT id FROM orders WHERE id IN ( SELECT id FROM customers ORDER BY name )",
        },
        SameReadingTestCase {
            description: "inside a derived table",
            sql: "SELECT x FROM ( ( SELECT id AS x FROM orders ) ORDER BY quantity ) d",
            expected_same_as: "SELECT x FROM ( SELECT id AS x FROM orders ORDER BY quantity ) d",
        },
        SameReadingTestCase {
            description: "a pipe ORDER BY after the parentheses",
            sql: "( SELECT id FROM orders ) |> ORDER BY id",
            expected_same_as: "SELECT id FROM orders |> ORDER BY id",
        },
        SameReadingTestCase {
            description: "common table expressions before the parentheses",
            sql: "WITH c AS ( SELECT id FROM orders ) ( SELECT id, nope FROM c )",
            expected_same_as: "WITH c AS ( SELECT id FROM orders ) SELECT id, nope FROM c",
        },
        SameReadingTestCase {
            description: "common table expressions before the parentheses and a clause after",
            sql: "WITH c AS ( SELECT id, quantity FROM orders ) ( SELECT id FROM c ) ORDER BY quantity",
            expected_same_as: "WITH c AS ( SELECT id, quantity FROM orders ) SELECT id FROM c ORDER BY quantity",
        },
        SameReadingTestCase {
            description: "common table expressions before the parentheses, a LIMIT inside and a clause after",
            sql: "WITH c AS ( SELECT id, quantity FROM orders ) ( SELECT id, nope FROM c LIMIT 1 ) ORDER BY id",
            expected_same_as: "( WITH c AS ( SELECT id, quantity FROM orders ) SELECT id, nope FROM c LIMIT 1 ) ORDER BY id",
        },
        SameReadingTestCase {
            description: "a LIMIT inside and an unknown ORDER BY column after",
            sql: "( SELECT id FROM orders LIMIT 1 ) ORDER BY nope",
            expected_same_as: "SELECT id FROM orders ORDER BY nope LIMIT 1",
        },
        SameReadingTestCase {
            description: "common table expressions before the parentheses, a LIMIT inside and an unknown column after",
            sql: "WITH c AS ( SELECT id FROM orders ) ( SELECT id FROM c LIMIT 1 ) ORDER BY nope",
            expected_same_as: "WITH c AS ( SELECT id FROM orders ) SELECT id FROM c ORDER BY nope LIMIT 1",
        },
        SameReadingTestCase {
            description: "common table expressions inside the parentheses, a LIMIT inside and an unknown column after",
            sql: "( WITH c AS ( SELECT id FROM orders ) SELECT id FROM c LIMIT 1 ) ORDER BY nope",
            expected_same_as: "WITH c AS ( SELECT id FROM orders ) SELECT id FROM c ORDER BY nope LIMIT 1",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            reading(test_case.sql),
            reading(test_case.expected_same_as),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_named_window_when_checked_then_an_undefined_name_or_unknown_key_is_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "the window function inside reads the window named after the parentheses",
            sql: "( SELECT sum ( quantity ) OVER w AS x FROM orders LIMIT 1 ) WINDOW w AS ( ORDER BY quantity )",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a window no WINDOW clause names is still unknown",
            sql: "( SELECT sum ( quantity ) OVER w2 AS x FROM orders LIMIT 1 ) WINDOW w AS ( ORDER BY quantity )",
            expected_findings: &[("E232", "Unknown named window 'w2'", 15)],
        },
        QueryFindingsTestCase {
            description: "an unknown partition key in the window named after the parentheses",
            sql: "( SELECT sum ( quantity ) OVER w AS x FROM orders LIMIT 1 ) WINDOW w AS ( PARTITION BY nope )",
            expected_findings: &[("E201", "Unknown column 'nope' in table 'orders'", 87)],
        },
        QueryFindingsTestCase {
            description: "an unknown order key in a WINDOW clause without parentheses",
            sql: "SELECT sum ( quantity ) OVER w AS x FROM orders WINDOW w AS ( ORDER BY nope )",
            expected_findings: &[("E201", "Unknown column 'nope' in table 'orders'", 71)],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            databricks_findings(test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_typed_literal_of_a_raw_string_when_analysed_then_it_reads_as_the_plain_one() {
    let test_cases = [
        SameReadingTestCase {
            description: "DATE",
            sql: "SELECT DATE r'2020-01-01' AS d",
            expected_same_as: "SELECT DATE '2020-01-01' AS d",
        },
        SameReadingTestCase {
            description: "TIMESTAMP",
            sql: "SELECT TIMESTAMP r'2020-01-01 10:00:00' AS d",
            expected_same_as: "SELECT TIMESTAMP '2020-01-01 10:00:00' AS d",
        },
        SameReadingTestCase {
            description: "TIMESTAMP_NTZ",
            sql: "SELECT TIMESTAMP_NTZ r'2020-01-01 10:00:00' AS d",
            expected_same_as: "SELECT TIMESTAMP_NTZ '2020-01-01 10:00:00' AS d",
        },
        SameReadingTestCase {
            description: "a hexadecimal binary",
            sql: "SELECT X r'AB' AS d",
            expected_same_as: "SELECT X'AB' AS d",
        },
        SameReadingTestCase {
            description: "an INTERVAL with a unit",
            sql: "SELECT INTERVAL r'1' DAY AS d",
            expected_same_as: "SELECT INTERVAL '1' DAY AS d",
        },
        SameReadingTestCase {
            description: "an INTERVAL string alone",
            sql: "SELECT INTERVAL r'1 day' AS d",
            expected_same_as: "SELECT INTERVAL '1 day' AS d",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            reading(test_case.sql),
            reading(test_case.expected_same_as),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_adjacent_strings_when_analysed_then_they_read_as_the_string_they_join_to() {
    let test_cases = [
        SameReadingTestCase {
            description: "an output's type",
            sql: "SELECT 'a' 'b' AS x",
            expected_same_as: "SELECT 'ab' AS x",
        },
        SameReadingTestCase {
            description: "a number column compared with a string that is not a number",
            sql: "SELECT id FROM orders WHERE quantity = 'x' 'y'",
            expected_same_as: "SELECT id FROM orders WHERE quantity = 'xy'",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            reading(test_case.sql),
            reading(test_case.expected_same_as),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_one_string_spelled_two_ways_when_grouping_then_the_expressions_match() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "adjacent strings in the select, one string in GROUP BY",
            sql: "SELECT COALESCE(status, 'a' 'b') AS x FROM orders GROUP BY COALESCE(status, 'ab')",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "one string in the select, adjacent strings in GROUP BY",
            sql: "SELECT COALESCE(status, 'ab') AS x FROM orders GROUP BY COALESCE(status, 'a' 'b')",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "adjacent strings in a concatenation",
            sql: "SELECT status || 'a' 'b' AS x FROM orders GROUP BY status || 'ab'",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a raw string in the select, a plain one in GROUP BY",
            sql: "SELECT COALESCE(status, r'ab') AS x FROM orders GROUP BY COALESCE(status, 'ab')",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a plain string in the select, a raw one in GROUP BY",
            sql: "SELECT COALESCE(status, 'ab') AS x FROM orders GROUP BY COALESCE(status, r'ab')",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "adjacent strings of another value are still a different expression",
            sql: "SELECT COALESCE(status, 'a' 'c') AS x FROM orders GROUP BY COALESCE(status, 'ab')",
            expected_findings: &[(
                "E230",
                "Column must be grouped or used in an aggregate function",
                16,
            )],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            databricks_findings(test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_comma_after_a_joined_relation_when_checked_then_the_join_reads_only_its_own_relations() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a USING key the left of its join lacks, a later comma relation offering it",
            sql: "SELECT * FROM customers a JOIN orders b USING (status), orders c",
            expected_findings: &[(
                "E201",
                "JOIN USING column 'status' must exist on both sides",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "a USING key both sides of its join offer, before a comma relation",
            sql: "SELECT * FROM orders a JOIN orders b USING (status), customers c",
            expected_findings: &[],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            databricks_findings(test_case.sql),
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_using_join_after_a_comma_when_expanding_a_star_then_its_key_leads_its_own_group() {
    let test_cases = [StarOrderTestCase {
        description: "the USING key comes first in its group, after the earlier comma relation",
        sql: "SELECT * FROM customers a, customers b JOIN customers c USING (name)",
        expected_names: &["id", "name", "name", "id", "id"],
    }];
    for test_case in test_cases {
        let names: Vec<String> = databricks_outputs(test_case.sql)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(names, test_case.expected_names, "{}", test_case.description);
    }
}
