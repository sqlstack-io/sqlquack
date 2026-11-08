//! The analysis probe: deep nestings parsed and then analysed, lineage and findings, over one table.

use sqlquack_semantics::dialects::constants::{DUCKDB, POSTGRES, SNOWFLAKE};
use sqlquack_semantics::dialects::types::DialectProfile;
use sqlquack_semantics::lineage::models::{LineageContext, Nullability, SchemaColumn};
use sqlquack_semantics::lineage::types::LineageSchema;
use sqlquack_semantics::{statement_lineage, statement_name_findings};
use sqlquack_syntax::{CallMode, Dialect, Expression, ParseOptions};

use crate::check::models::{LocalFailure, LocalVerdict};
use crate::depth::models::Construct;
use crate::formats::models::LocalParser;

/// The one relation the probed SQL reads: `orders`, with a key, a quantity and a status.
#[derive(Debug)]
struct Orders {
    relation: &'static str,
    columns: Vec<SchemaColumn>,
}

impl LineageSchema for Orders {
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        (relation == self.relation).then_some(self.columns.as_slice())
    }
}

/// `orders` as `parser`'s catalog names it, with its analysis profile; none without a profile.
fn orders(parser: LocalParser) -> Option<(Orders, &'static dyn DialectProfile)> {
    let column = |name: &str, data_type: &str| SchemaColumn {
        name: name.to_owned(),
        quoted: false,
        data_type: Some(data_type.to_owned()),
        nullability: Nullability::Unknown,
    };
    let (relation, columns, profile): (&str, Vec<SchemaColumn>, &'static dyn DialectProfile) =
        match parser {
            LocalParser::Snowflake => (
                "ORDERS",
                vec![
                    column("ID", "NUMBER(38,0)"),
                    column("QUANTITY", "NUMBER(38,0)"),
                    column("STATUS", "VARCHAR"),
                ],
                &SNOWFLAKE,
            ),
            LocalParser::DuckDb | LocalParser::Postgres => (
                "orders",
                vec![
                    column("id", "INTEGER"),
                    column("quantity", "INTEGER"),
                    column("status", "VARCHAR"),
                ],
                match parser {
                    LocalParser::DuckDb => &DUCKDB,
                    _ => &POSTGRES,
                },
            ),
            LocalParser::BigQuery
            | LocalParser::Databricks
            | LocalParser::SqlServer
            | LocalParser::None => return None,
        };
    Some((Orders { relation, columns }, profile))
}

/// The statements `parser` reads from `sql`, or its verdict when it reads none.
fn parsed(parser: LocalParser, sql: &str) -> Result<Vec<Expression>, LocalVerdict> {
    match parser {
        LocalParser::Snowflake => sqlquack_syntax::parse_statements_with(
            sql,
            Dialect::Snowflake,
            &ParseOptions::default(),
            CallMode::Faithful,
        )
        .map_err(|error| match error {
            sqlquack_syntax::Error::Parse { message, .. } if message.starts_with("E_GUARD_") => {
                LocalVerdict::Failed {
                    failure: LocalFailure::Guard,
                }
            }
            _ => LocalVerdict::Rejected,
        }),
        LocalParser::DuckDb => sqlquack_syntax_duckdb::parse_strict(sql)
            .map(|parsed| parsed.statements)
            .map_err(|_| LocalVerdict::Rejected),
        LocalParser::Postgres => sqlquack_syntax_postgres::parse_strict(sql)
            .map(|parsed| parsed.statements)
            .map_err(|_| LocalVerdict::Rejected),
        LocalParser::BigQuery
        | LocalParser::Databricks
        | LocalParser::SqlServer
        | LocalParser::None => Err(LocalVerdict::Failed {
            failure: LocalFailure::NoParser,
        }),
    }
}

/// `sql` parsed by `parser`, then each statement's lineage and findings under its profile.
fn analysed_verdict(parser: LocalParser, sql: &str) -> LocalVerdict {
    let (statements, (schema, profile)) = match (parsed(parser, sql), orders(parser)) {
        (Ok(statements), Some(orders)) => (statements, orders),
        (Err(verdict), _) => return verdict,
        (Ok(_), None) => {
            return LocalVerdict::Failed {
                failure: LocalFailure::NoParser,
            };
        }
    };
    let context = LineageContext {
        profile,
        schema: &schema,
        sql: Some(sql),
    };
    for statement in &statements {
        statement_lineage(statement, context);
        statement_name_findings(statement, context);
    }
    LocalVerdict::Accepted {
        opaque: false,
        tree_sha256: None,
    }
}

/// SQLQuack's Snowflake verdict on `sql`, which it then analyses.
pub(crate) fn snowflake_analysis(sql: &str) -> LocalVerdict {
    analysed_verdict(LocalParser::Snowflake, sql)
}

/// SQLQuack's DuckDB verdict on `sql`, which it then analyses.
pub(crate) fn duckdb_analysis(sql: &str) -> LocalVerdict {
    analysed_verdict(LocalParser::DuckDb, sql)
}

/// SQLQuack's PostgreSQL verdict on `sql`, which it then analyses.
pub(crate) fn postgres_analysis(sql: &str) -> LocalVerdict {
    analysed_verdict(LocalParser::Postgres, sql)
}

/// `shape` at `depth` for a parser with an analysis profile.
fn analysed(parser: LocalParser, depth: usize, shape: fn(usize) -> String) -> Option<String> {
    orders(parser).is_some().then(|| shape(depth))
}

fn derived_tables(depth: usize) -> String {
    (0..depth).fold("SELECT id FROM orders".to_owned(), |query, _| {
        format!("SELECT * FROM ({query}) AS t")
    })
}

fn filtered_derived_tables(depth: usize) -> String {
    (0..depth).fold("SELECT id FROM orders".to_owned(), |query, _| {
        format!("SELECT * FROM ({query}) AS t WHERE t.id > 0")
    })
}

fn union_chain(depth: usize) -> String {
    (0..depth).fold("SELECT id FROM orders".to_owned(), |query, _| {
        format!("SELECT id FROM orders UNION ALL SELECT * FROM ({query}) AS t")
    })
}

fn parenthesised_unions(depth: usize) -> String {
    (0..depth).fold("SELECT id FROM orders".to_owned(), |query, _| {
        format!("SELECT * FROM ((SELECT id FROM orders) UNION ALL ({query})) AS t")
    })
}

fn joins(depth: usize) -> String {
    let joined: String = (1..=depth)
        .map(|index| {
            format!(
                " JOIN orders AS o{index} ON o{index}.id = o{}.id",
                index - 1
            )
        })
        .collect();
    format!("SELECT o0.id FROM orders AS o0{joined}")
}

fn nested_case(depth: usize) -> String {
    let value: String = (0..depth).fold("id".to_owned(), |value, _| {
        format!("CASE WHEN quantity > 0 THEN {value} ELSE 0 END")
    });
    format!("SELECT {value} AS x FROM orders")
}

fn nested_arithmetic(depth: usize) -> String {
    let value: String = (0..depth).fold("id".to_owned(), |value, _| format!("({value} + 1)"));
    format!("SELECT {value} AS x FROM orders")
}

fn scalar_subqueries(depth: usize) -> String {
    let value: String = (0..depth).fold("id".to_owned(), |value, _| {
        format!("(SELECT {value} FROM orders AS o LIMIT 1)")
    });
    format!("SELECT {value} AS x FROM orders")
}

fn cte_chain(depth: usize) -> String {
    let ctes: String = (1..=depth)
        .map(|index| format!(", c{index} AS (SELECT id FROM c{})", index - 1))
        .collect();
    format!("WITH c0 AS (SELECT id FROM orders){ctes} SELECT id FROM c{depth}")
}

/// Every analysed construct, each probed in turn with `analysis_verdict`, in report order.
pub(crate) fn analysed_constructs() -> Vec<Construct> {
    vec![
        Construct {
            name: "analysed_derived_tables",
            text: |parser, depth| analysed(parser, depth, derived_tables),
        },
        Construct {
            name: "analysed_filtered_derived_tables",
            text: |parser, depth| analysed(parser, depth, filtered_derived_tables),
        },
        Construct {
            name: "analysed_union_chain",
            text: |parser, depth| analysed(parser, depth, union_chain),
        },
        Construct {
            name: "analysed_parenthesised_unions",
            text: |parser, depth| analysed(parser, depth, parenthesised_unions),
        },
        Construct {
            name: "analysed_joins",
            text: |parser, depth| analysed(parser, depth, joins),
        },
        Construct {
            name: "analysed_case",
            text: |parser, depth| analysed(parser, depth, nested_case),
        },
        Construct {
            name: "analysed_arithmetic",
            text: |parser, depth| analysed(parser, depth, nested_arithmetic),
        },
        Construct {
            name: "analysed_scalar_subqueries",
            text: |parser, depth| analysed(parser, depth, scalar_subqueries),
        },
        Construct {
            name: "analysed_cte_chain",
            text: |parser, depth| analysed(parser, depth, cte_chain),
        },
    ]
}
