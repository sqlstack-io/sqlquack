use crate::duckdb::profile::tests::test_types::{
    AliasReadTestCase, ResolvedReadTestCase, SingleAnalysisTestCase,
};

use crate::lineage::main::statement_lineage::statement_lineage;
use std::collections::HashMap;

use crate::lineage::tests::helpers::{typed_column, typed_duckdb_shop_schema};
use crate::lineage::tests::test_types::TypedShopSchema;

use sqlquack_parse::parse_statements_with;
use sqlquack_syntax::{CallMode, Dialect, ParseOptions};

use crate::dialects::constants::DUCKDB;
use crate::lineage::_helpers::tracing::reuse::work_done;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::main::statement_lineage_and_findings::statement_lineage_and_findings;
use crate::lineage::main::statement_name_findings::statement_name_findings;
use crate::lineage::models::{LineageContext, TransformKind};
use crate::lineage::tests::helpers::shop_schema;
use crate::lineage::tests::test_types::QueryLineageTestCase;

#[test]
fn given_a_query_when_tracing_lineage_then_each_output_names_its_sources_and_transform() {
    let test_cases = [
        QueryLineageTestCase {
            description: "a derived table without LATERAL reads the item before it, as DuckDB runs it",
            sql: "SELECT k.n FROM orders AS p, (SELECT p.status AS n) AS k",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[("n", TransformKind::Direct, &["orders.status"])],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "DuckDB names a pivoted column by its value's text, without quotes",
            sql: "SELECT * FROM (SELECT customer_id, status, amount FROM orders) PIVOT (SUM(amount) FOR status IN ('open', 'shipped'))",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                (
                    "customer_id",
                    TransformKind::Direct,
                    &["orders.customer_id"],
                ),
                ("open", TransformKind::Aggregation, &["orders.amount"]),
                ("shipped", TransformKind::Aggregation, &["orders.amount"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a DuckDB PIVOT with GROUP BY passes only the grouped columns through",
            sql: "SELECT * FROM (SELECT id, customer_id, status, amount FROM orders) \
                  PIVOT (SUM(amount) FOR status IN ('open') GROUP BY customer_id)",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                (
                    "customer_id",
                    TransformKind::Direct,
                    &["orders.customer_id"],
                ),
                ("open", TransformKind::Aggregation, &["orders.amount"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "a DuckDB PIVOT statement keeps its GROUP BY columns; its pivoted ones are unnamed",
            sql: "PIVOT (SELECT id, customer_id, status, amount FROM orders) ON status USING SUM(amount) \
                  GROUP BY customer_id",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[(
                "customer_id",
                TransformKind::Direct,
                &["orders.customer_id"],
            )],
            expected_open_star: true,
        },
        QueryLineageTestCase {
            description: "DuckDB names VALUES columns col0 onwards and ignores case",
            sql: "SELECT v.col0 AS first, id FROM (VALUES (1)) v, ORDERS",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                ("first", TransformKind::Constant, &[]),
                ("id", TransformKind::Direct, &["ORDERS.ID"]),
            ],
            expected_open_star: false,
        },
        QueryLineageTestCase {
            description: "DuckDB names an unaliased number by its text",
            sql: "SELECT 1, id FROM ORDERS",
            dialect: Dialect::DuckDB,
            profile: &DUCKDB,
            expected_outputs: &[
                ("1", TransformKind::Constant, &[]),
                ("id", TransformKind::Direct, &["ORDERS.ID"]),
            ],
            expected_open_star: false,
        },
    ];
    let schema = shop_schema();
    for test_case in test_cases {
        let context = LineageContext {
            profile: test_case.profile,
            schema: &schema,
            sql: None,
        };
        let lineage = query_lineage(
            test_case.sql,
            test_case.dialect,
            &ParseOptions::default(),
            context,
        )
        .unwrap_or_else(|error| panic!("{}: {error}", test_case.description));
        let outputs: Vec<(String, TransformKind, Vec<String>)> = lineage
            .outputs
            .iter()
            .map(|output| {
                (
                    output.name.clone(),
                    output.kind,
                    output
                        .sources
                        .iter()
                        .map(|source| format!("{}.{}", source.relation, source.column))
                        .collect(),
                )
            })
            .collect();
        let expected: Vec<(String, TransformKind, Vec<String>)> = test_case
            .expected_outputs
            .iter()
            .map(|(name, kind, sources)| {
                (
                    (*name).to_owned(),
                    *kind,
                    sources.iter().map(|source| (*source).to_owned()).collect(),
                )
            })
            .collect();
        assert_eq!(outputs, expected, "{}", test_case.description);
        assert_eq!(
            lineage.open_star, test_case.expected_open_star,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_duckdb_value_keyword_beside_an_open_relation_when_tracing_lineage_then_it_reads_no_column()
 {
    let test_cases = [
        ResolvedReadTestCase {
            description: "CURRENT_TIMESTAMP is DuckDB's function, not a column of a star over an open relation",
            sql: "WITH src AS (SELECT * FROM raw_orders) SELECT CURRENT_TIMESTAMP AS loaded_at FROM src",
            expected_sources: &[],
            expected_resolved: true,
        },
        ResolvedReadTestCase {
            description: "current_date in lower case is the function too",
            sql: "WITH src AS (SELECT * FROM raw_orders) SELECT current_date AS loaded_on FROM src",
            expected_sources: &[],
            expected_resolved: true,
        },
        ResolvedReadTestCase {
            description: "a qualified keyword is a column of the star over the open relation",
            sql: "WITH src AS (SELECT * FROM raw_orders) SELECT src.current_date AS loaded_on FROM src",
            expected_sources: &["raw_orders.current_date"],
            expected_resolved: true,
        },
        ResolvedReadTestCase {
            description: "an ordinary name is a column of the star over the open relation",
            sql: "WITH src AS (SELECT * FROM raw_orders) SELECT status FROM src",
            expected_sources: &["raw_orders.status"],
            expected_resolved: true,
        },
    ];
    let mut schema: TypedShopSchema = typed_duckdb_shop_schema();
    schema.open.insert("raw_orders".to_owned(), Vec::new());
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
        let lineage = statement_lineage(&statement, context);
        let output = &lineage.outputs[0];
        let sources: Vec<String> = output
            .sources
            .iter()
            .map(|source| format!("{}.{}", source.relation, source.column))
            .collect();
        assert_eq!(
            (sources, output.resolved),
            (
                test_case
                    .expected_sources
                    .iter()
                    .map(|source| (*source).to_owned())
                    .collect::<Vec<String>>(),
                test_case.expected_resolved
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_ctes_when_analysing_lineage_and_findings_together_then_the_lineage_reuses_the_resolved_ctes()
 {
    let test_cases = [
        SingleAnalysisTestCase {
            description: "a chain of CTEs read through a join and a union",
            sql: "WITH a AS (SELECT id, amount FROM orders), \
                  b AS (SELECT a.id, a.amount + 1 AS amount FROM a JOIN customers AS c ON c.id = a.id), \
                  u AS (SELECT id, amount FROM a UNION ALL SELECT id, amount FROM b) \
                  SELECT id, SUM(amount) AS total FROM u GROUP BY id",
            expected_outputs: &["id", "total"],
            expected_finding_count: 0,
        },
        SingleAnalysisTestCase {
            description: "a query reading a column its CTE lacks, so there is a finding to keep",
            sql: "WITH a AS (SELECT id, amount FROM orders), \
                  b AS (SELECT a.id, a.amount FROM a JOIN customers AS c ON c.id = a.id) \
                  SELECT id, missing FROM b",
            expected_outputs: &["id", "missing"],
            expected_finding_count: 1,
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
        work_done();
        let findings = statement_name_findings(&statements[0], context);
        let findings_work: usize = work_done();
        let lineage = statement_lineage(&statements[0], context);
        let lineage_work: usize = work_done();
        let together = statement_lineage_and_findings(&statements[0], context);
        let together_work: usize = work_done();
        assert_eq!(together, (lineage, findings), "{}", test_case.description);
        assert_eq!(
            (
                together
                    .0
                    .outputs
                    .iter()
                    .map(|output| output.name.as_str())
                    .collect::<Vec<&str>>(),
                together.1.len()
            ),
            (
                test_case.expected_outputs.to_vec(),
                test_case.expected_finding_count
            ),
            "{}",
            test_case.description
        );
        assert!(
            (together_work - findings_work) * 2 <= lineage_work,
            "{}: lineage alone {lineage_work}, after the findings {}",
            test_case.description,
            together_work - findings_work
        );
    }
}

#[test]
fn given_select_aliases_read_at_several_levels_when_analysing_together_then_lineage_and_findings_match_separate_runs()
 {
    let test_cases = [
        AliasReadTestCase {
            description: "a later projection reads an earlier alias at the top level",
            sql: "SELECT price * 2 AS doubled, doubled + 1 AS again FROM orders",
            expected_sources: &[("doubled", &["orders.price"]), ("again", &["orders.price"])],
            expected_finding_count: 0,
        },
        AliasReadTestCase {
            description: "an alias spelled like a source column, which the source column shadows",
            sql: "SELECT quantity + 1 AS id, id * 2 AS twice FROM orders",
            expected_sources: &[("id", &["orders.quantity"]), ("twice", &["orders.id"])],
            expected_finding_count: 0,
        },
        AliasReadTestCase {
            description: "a text alias read in arithmetic inside a CTE body",
            sql: "WITH c AS (SELECT status AS label, label + 1 AS bumped FROM orders) SELECT bumped FROM c",
            expected_sources: &[("bumped", &["orders.status"])],
            expected_finding_count: 0,
        },
        AliasReadTestCase {
            description: "a scalar subquery reads the enclosing level's alias",
            sql: "SELECT price AS cost, (SELECT cost + c.id FROM customers AS c LIMIT 1) AS nested FROM orders",
            expected_sources: &[
                ("cost", &["orders.price"]),
                ("nested", &["customers.id", "orders.price"]),
            ],
            expected_finding_count: 0,
        },
        AliasReadTestCase {
            description: "a derived table's alias read by its later projection and by the level above",
            sql: "SELECT again FROM (SELECT price AS cost, cost * 2 AS again FROM orders) AS d WHERE again > 1",
            expected_sources: &[("again", &["orders.price"])],
            expected_finding_count: 0,
        },
    ];
    let schema: TypedShopSchema = typed_duckdb_shop_schema();
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
        let separate = (
            statement_lineage(&statement, context),
            statement_name_findings(&statement, context),
        );
        let together = statement_lineage_and_findings(&statement, context);
        assert_eq!(together, separate, "{}", test_case.description);
        let sources: Vec<(String, Vec<String>)> = together
            .0
            .outputs
            .iter()
            .map(|output| {
                (
                    output.name.clone(),
                    output
                        .sources
                        .iter()
                        .map(|source| format!("{}.{}", source.relation, source.column))
                        .collect(),
                )
            })
            .collect();
        let expected_sources: Vec<(String, Vec<String>)> = test_case
            .expected_sources
            .iter()
            .map(|(name, columns)| {
                (
                    (*name).to_owned(),
                    columns.iter().map(|column| (*column).to_owned()).collect(),
                )
            })
            .collect();
        assert_eq!(
            (sources, together.1.len()),
            (expected_sources, test_case.expected_finding_count),
            "{}: findings {:?}",
            test_case.description,
            together.1
        );
    }
}

#[test]
fn given_union_branches_of_one_type_spelled_two_ways_when_tracing_duckdb_lineage_then_no_cast_is_reported()
 {
    let test_cases = [QueryLineageTestCase {
        description: "INT4 and INTEGER spell one DuckDB type, so the union converts nothing",
        sql: "SELECT id FROM orders UNION ALL SELECT customer_id FROM orders",
        dialect: Dialect::DuckDB,
        profile: &DUCKDB,
        expected_outputs: &[(
            "id",
            TransformKind::Direct,
            &["orders.customer_id", "orders.id"],
        )],
        expected_open_star: false,
    }];
    let schema = TypedShopSchema {
        closed: HashMap::from([(
            "orders".to_owned(),
            vec![
                typed_column("id", "INT4"),
                typed_column("customer_id", "INTEGER"),
            ],
        )]),
        ..TypedShopSchema::default()
    };
    for test_case in test_cases {
        let context = LineageContext {
            profile: test_case.profile,
            schema: &schema,
            sql: None,
        };
        let traced: Vec<(String, TransformKind, Vec<String>)> = query_lineage(
            test_case.sql,
            test_case.dialect,
            &ParseOptions::default(),
            context,
        )
        .unwrap_or_else(|error| panic!("{}: {error}", test_case.description))
        .outputs
        .iter()
        .map(|output| {
            let sources = output.sources.iter();
            (
                output.name.clone(),
                output.kind,
                sources
                    .map(|source| format!("{}.{}", source.relation, source.column))
                    .collect(),
            )
        })
        .collect();
        let traced: Vec<(&str, TransformKind, Vec<&str>)> = traced
            .iter()
            .map(|(name, kind, sources)| {
                (
                    name.as_str(),
                    *kind,
                    sources.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            traced,
            test_case
                .expected_outputs
                .iter()
                .map(|(name, kind, sources)| (*name, *kind, sources.to_vec()))
                .collect::<Vec<_>>(),
            "{}",
            test_case.description
        );
    }
}
