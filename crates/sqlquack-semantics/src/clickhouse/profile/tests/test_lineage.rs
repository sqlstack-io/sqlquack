use sqlquack_syntax::Expression;

use crate::clickhouse::profile::tests::helpers::clickhouse_join_schema;
use crate::clickhouse::profile::tests::helpers::clickhouse_shop_schema;
use crate::clickhouse::profile::tests::helpers::parsed_clickhouse;
use crate::clickhouse::profile::tests::helpers::replayed_clickhouse_engine_types;
use crate::clickhouse::profile::tests::test_types::ClickhouseJoinTestCase;
use crate::clickhouse::profile::tests::test_types::ClickhouseTypeTestCase;
use crate::dialects::constants::CLICKHOUSE;
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::models::Nullability::{NonNull, Nullable, Unknown};
use crate::lineage::tests::helpers::duckdb_shop_schema;
use crate::lineage::tests::helpers::output_ranges;
use crate::lineage::tests::helpers::owned_findings;
use crate::lineage::tests::test_types::StrictRangeTestCase;
use crate::lineage::tests::test_types::{EngineTypeReplayTestCase, QueryFindingsTestCase};

/// Result types ClickHouse 26.1 reported for built-in calls and expressions, under SQLBuild's settings.
const ENGINE_TYPES: &str = include_str!("fixtures/clickhouse_engine_types.json");

#[test]
fn given_clickhouse_values_when_typing_outputs_then_each_has_the_engine_type() {
    let test_cases = [
        ClickhouseTypeTestCase {
            description: "an integer literal takes the narrowest unsigned type that holds it",
            sql: "SELECT 255 AS v FROM orders",
            expected_output: ("v", Some("UInt8"), NonNull),
        },
        ClickhouseTypeTestCase {
            description: "one past UInt8 is UInt16",
            sql: "SELECT 256 AS v FROM orders",
            expected_output: ("v", Some("UInt16"), NonNull),
        },
        ClickhouseTypeTestCase {
            description: "one past Int8 below zero is Int16",
            sql: "SELECT -129 AS v FROM orders",
            expected_output: ("v", Some("Int16"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "count is UInt64",
            sql: "SELECT count() AS v FROM orders",
            expected_output: ("v", Some("UInt64"), NonNull),
        },
        ClickhouseTypeTestCase {
            description: "sum of Int32 widens to Int64",
            sql: "SELECT sum(quantity) AS v FROM orders",
            expected_output: ("v", Some("Int64"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "Int32 plus a UInt8 literal is the next wider signed type",
            sql: "SELECT quantity + 1 AS v FROM orders",
            expected_output: ("v", Some("Int64"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "subtracting unsigned literals is signed",
            sql: "SELECT 1 - 1 AS v FROM orders",
            expected_output: ("v", Some("Int16"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "division is always Float64",
            sql: "SELECT quantity / 2 AS v FROM orders",
            expected_output: ("v", Some("Float64"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "abs of a signed type is the unsigned type of the same width",
            sql: "SELECT abs(quantity) AS v FROM orders",
            expected_output: ("v", Some("UInt32"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "negating an unsigned type widens it to signed",
            sql: "SELECT -abs(quantity) AS v FROM orders",
            expected_output: ("v", Some("Int64"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "toDecimal256 with a scale is Decimal(76, scale)",
            sql: "SELECT toDecimal256('2.4700', 4) AS v FROM orders",
            expected_output: ("v", Some("Decimal(76, 4)"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "a float times a Decimal is Float64",
            sql: "SELECT toFloat32(quantity) * toDecimal32(1, 2) AS v FROM orders",
            expected_output: ("v", Some("Float64"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "an integer times a Decimal follows rules the profile leaves untyped",
            sql: "SELECT quantity * toDecimal64(1, 2) AS v FROM orders",
            expected_output: ("v", None, Unknown),
        },
        ClickhouseTypeTestCase {
            description: "a comparison is UInt8",
            sql: "SELECT status LIKE 'a%' AS v FROM orders",
            expected_output: ("v", Some("UInt8"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "if takes the common type of its branches",
            sql: "SELECT if(quantity > 1, 1, 300) AS v FROM orders",
            expected_output: ("v", Some("UInt16"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "CASE over UInt8 and Int8 takes the signed type holding both",
            sql: "SELECT CASE WHEN quantity > 1 THEN 1 ELSE -1 END AS v FROM orders",
            expected_output: ("v", Some("Int16"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "a cast to a SQL alias takes ClickHouse's type name",
            sql: "SELECT CAST(quantity AS BIGINT UNSIGNED) AS v FROM orders",
            expected_output: ("v", Some("UInt64"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "a cast keeps a zoned DateTime64's arguments, written `, `-separated",
            sql: "SELECT CAST(status AS DateTime64( 3,'UTC' )) AS v FROM orders",
            expected_output: ("v", Some("DateTime64(3, 'UTC')"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "an integer alias ignores its display width",
            sql: "SELECT CAST(quantity AS INT(11)) AS v FROM orders",
            expected_output: ("v", Some("Int32"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "FLOAT ignores its precision, even one past 24",
            sql: "SELECT CAST(quantity AS FLOAT(53)) AS v FROM orders",
            expected_output: ("v", Some("Float32"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "BINARY without a length is no complete type",
            sql: "SELECT CAST(status AS BINARY) AS v FROM orders",
            expected_output: ("v", None, Unknown),
        },
        ClickhouseTypeTestCase {
            description: "a cast keeps decimal parameters in ClickHouse's spelling",
            sql: "SELECT CAST(price AS DECIMAL(12, 4)) AS v FROM orders",
            expected_output: ("v", Some("Decimal(12, 4)"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "a UNION ALL branch's literal widens to the column's type",
            sql: "SELECT id AS v FROM orders UNION ALL SELECT 1 FROM orders",
            expected_output: ("v", Some("Int32"), Unknown),
        },
        ClickhouseTypeTestCase {
            description: "the shared column of a RIGHT JOIN is the left one, nullable",
            sql: "SELECT id AS v FROM orders RIGHT JOIN customers ON orders.id = customers.id",
            expected_output: ("v", Some("Int32"), Nullable),
        },
    ];
    let schema = clickhouse_shop_schema();
    for test_case in test_cases {
        let statement: Expression = parsed_clickhouse(test_case.sql);
        let context = LineageContext {
            profile: &CLICKHOUSE,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let lineage = statement_lineage(&statement, context);
        let output = &lineage.outputs[0];
        assert_eq!(
            (
                output.name.as_str(),
                output.data_type.as_deref(),
                output.nullability
            ),
            test_case.expected_output,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_clickhouse_names_when_checking_a_query_then_only_engine_errors_are_found() {
    let test_cases = [
        QueryFindingsTestCase {
            description: "a name both sides of the one join offer is the left side's (26.1: accepted)",
            sql: "SELECT id FROM orders JOIN customers ON orders.id = customers.id",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a name three joined tables offer is ambiguous (26.1: AMBIGUOUS_IDENTIFIER)",
            sql: "SELECT id FROM orders JOIN customers ON orders.id = customers.id \
                  JOIN orders AS o2 ON o2.id = orders.id",
            expected_findings: &[(
                "E221",
                "Ambiguous unqualified column 'id' found in 3 referenced tables",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "names are case-sensitive (26.1: UNKNOWN_IDENTIFIER)",
            sql: "SELECT Status AS v FROM orders",
            expected_findings: &[("E201", "Unknown column 'Status' in table 'orders'", 7)],
        },
        QueryFindingsTestCase {
            description: "WHERE reads an output alias (26.1: accepted)",
            sql: "SELECT quantity AS q FROM orders WHERE q > 1",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a qualifier naming nothing over an open relation (26.1: UNKNOWN_IDENTIFIER)",
            sql: "SELECT zq.id FROM raw AS o",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "named false finding, shared with Polyglot: any subcolumn read of an open \
                          relation's unknown column (Nested, dotted, Tuple, JSON, Map, Variant); \
                          a Nested `zq` or a `zq.id` column (26.1 with one: accepted)",
            sql: "SELECT zq.id FROM raw",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "named false finding: a Tuple column `tup` (26.1 with one: accepted)",
            sql: "SELECT tup.a FROM raw",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'tup' referenced by column 'a'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "named false finding: a JSON column `j` (26.1 with one: accepted)",
            sql: "SELECT j.k FROM raw",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'j' referenced by column 'k'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "the same with a closed relation joined (26.1: UNKNOWN_IDENTIFIER)",
            sql: "SELECT zq.id FROM raw AS o JOIN orders AS p ON o.id = p.id",
            expected_findings: &[(
                "E222",
                "Unknown table or alias 'zq' referenced by column 'id'",
                7,
            )],
        },
        QueryFindingsTestCase {
            description: "an open relation's alias qualifies its columns (26.1: accepted)",
            sql: "SELECT o.id FROM raw AS o",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an aliased table's own name still qualifies it (26.1: accepted)",
            sql: "SELECT raw.id, orders.id FROM raw AS o JOIN orders AS p ON o.id = p.id",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a lambda parameter's tuple element (26.1: accepted)",
            sql: "SELECT arrayMap(x -> x.a, arr) AS v FROM nested",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a tuple column's element (26.1: accepted)",
            sql: "SELECT tup.a FROM nested",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a dotted column the catalogue knows (26.1: accepted)",
            sql: "SELECT zq.order_id FROM nested",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "an output alias's tuple element (26.1: accepted)",
            sql: "SELECT CAST((1, 'b'), 'Tuple(a Int64, b String)') AS t2, t2.a FROM raw",
            expected_findings: &[],
        },
        QueryFindingsTestCase {
            description: "a WITH alias's tuple element (26.1: accepted)",
            sql: "WITH CAST((1, 'b'), 'Tuple(a Int64, b String)') AS t2 SELECT t2.a FROM raw",
            expected_findings: &[],
        },
    ];
    let schema = clickhouse_shop_schema();
    for test_case in test_cases {
        let statement: Expression = parsed_clickhouse(test_case.sql);
        let context = LineageContext {
            profile: &CLICKHOUSE,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let found: Vec<(&'static str, String, usize)> = statement_findings(&statement, context)
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
            owned_findings(test_case.expected_findings),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_engine_recorded_clickhouse_types_when_typing_outputs_then_none_is_wrong() {
    let test_cases = [EngineTypeReplayTestCase {
        description: "every recorded ClickHouse 26.1 result type, replayed through the native parser",
        fixture: ENGINE_TYPES,
        replayed: replayed_clickhouse_engine_types,
        expected_wrong: &[],
        expected_minimum_typed: 249,
        expected_maximum_untyped: 2,
    }];
    for test_case in test_cases {
        let replay = (test_case.replayed)(test_case.fixture);
        let untyped: usize = replay.tally.values().map(|tally| tally.untyped).sum();
        eprintln!(
            "typed {} untyped {untyped} wrong {:?}",
            replay.typed, replay.wrong
        );
        assert_eq!(
            (
                replay.wrong.as_slice(),
                replay.typed >= test_case.expected_minimum_typed,
                untyped <= test_case.expected_maximum_untyped
            ),
            (test_case.expected_wrong, true, true),
            "{}: {:?}",
            test_case.description,
            replay.tally
        );
    }
}

#[test]
fn given_a_name_joined_relations_share_when_resolving_then_clickhouse_reads_the_left_one() {
    let test_cases = [
        ClickhouseJoinTestCase {
            description: "a non-key name of an inner join is the left side's (26.1: Int32)",
            sql: "SELECT v FROM a JOIN b ON a.id = b.id",
            expected_output: (Some("Int32"), Unknown),
            expected_codes: &[],
        },
        ClickhouseJoinTestCase {
            description: "a comma join reads the left side too (26.1: Int32)",
            sql: "SELECT v FROM a, b",
            expected_output: (Some("Int32"), Unknown),
            expected_codes: &[],
        },
        ClickhouseJoinTestCase {
            description: "a FULL join's left side is nullable (26.1: Nullable(Int32))",
            sql: "SELECT v FROM a FULL JOIN b ON a.id = b.id",
            expected_output: (Some("Int32"), Nullable),
            expected_codes: &[],
        },
        ClickhouseJoinTestCase {
            description: "a CTE on the right side does not make the name ambiguous (26.1: Int32)",
            sql: "WITH s AS (SELECT id, v FROM b) SELECT v FROM a JOIN s ON a.id = s.id",
            expected_output: (Some("Int32"), Unknown),
            expected_codes: &[],
        },
        ClickhouseJoinTestCase {
            description: "a self-join reads the left copy (26.1: Int32)",
            sql: "SELECT v FROM a JOIN a AS a2 ON a.id = a2.id",
            expected_output: (Some("Int32"), Unknown),
            expected_codes: &[],
        },
        ClickhouseJoinTestCase {
            description: "two of three joined tables offering the name is ambiguous (26.1: AMBIGUOUS_IDENTIFIER)",
            sql: "SELECT v FROM a JOIN b ON a.id = b.id JOIN c ON c.z = a.id",
            expected_output: (Some("Int32"), Unknown),
            expected_codes: &["E221"],
        },
    ];
    let schema = clickhouse_join_schema();
    for test_case in test_cases {
        let statement: Expression = parsed_clickhouse(test_case.sql);
        let context = LineageContext {
            profile: &CLICKHOUSE,
            schema: &schema,
            sql: Some(test_case.sql),
        };
        let lineage = statement_lineage(&statement, context);
        let output = &lineage.outputs[0];
        let codes: Vec<&str> = statement_findings(&statement, context)
            .into_iter()
            .map(|finding| finding.code)
            .collect();
        assert_eq!(
            (
                (output.data_type.as_deref(), output.nullability),
                codes.as_slice()
            ),
            (test_case.expected_output, test_case.expected_codes),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_strict_parse_when_tracing_lineage_then_each_output_has_its_items_byte_range() {
    let test_cases = [StrictRangeTestCase {
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
    }];
    for test_case in test_cases {
        let statement = sqlquack_syntax_clickhouse::parse_strict(test_case.sql)
            .unwrap_or_else(|failure| panic!("{}: {failure:?}", test_case.description))
            .statements
            .remove(0);
        assert_eq!(
            output_ranges(
                &statement,
                &CLICKHOUSE,
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
