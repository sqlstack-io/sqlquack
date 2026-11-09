use std::collections::HashMap;

use sqlquack_syntax::{CallMode, Dialect, ParseOptions};
use sqlquack_syntax_snowflake::parse_statements_with;

use crate::dialects::constants::SQLITE;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::models::Nullability;
use crate::lineage::tests::helpers::typed_column;
use crate::lineage::tests::test_types::TypedShopSchema;

/// Each SQLite output's name, type and nullability.
pub(crate) fn sqlite_outputs(
    schema: &TypedShopSchema,
    sql: &str,
) -> Vec<(String, Option<String>, Nullability)> {
    let context = LineageContext {
        profile: &SQLITE,
        schema,
        sql: Some(sql),
    };
    query_lineage(sql, Dialect::SQLite, &ParseOptions::default(), context)
        .unwrap_or_else(|error| panic!("{sql}: {error}"))
        .outputs
        .into_iter()
        .map(|output| (output.name, output.data_type, output.nullability))
        .collect()
}

/// `sql`'s statement with its first output's call kept generic, as the native SQLite parser writes calls.
pub(crate) fn generic_calls(sql: &str) -> sqlquack_syntax::Expression {
    let written: String = sql.replacen("SELECT ", "SELECT zz_", 1);
    let mut statements = parse_statements_with(
        &written,
        Dialect::SQLite,
        &ParseOptions::default(),
        CallMode::FaithfulLocated,
    )
    .unwrap_or_else(|error| panic!("{sql}: {error}"));
    let mut statement = statements.remove(0);
    let sqlquack_syntax::Expression::Select(select) = &mut statement else {
        panic!("{sql}: not a SELECT");
    };
    let Some(sqlquack_syntax::Expression::Alias(alias)) = select.expressions.first_mut() else {
        panic!("{sql}: no aliased first output");
    };
    let sqlquack_syntax::Expression::Function(function) = &mut alias.this else {
        panic!("{sql}: the first output is not a call");
    };
    function.name = function.name.trim_start_matches("zz_").to_owned();
    statement
}

/// `sql`'s tree depth, outputs and findings, analysed on a 2 MiB thread, or the parser's refusal.
pub(crate) fn deep_sqlite_analysis(sql: String) -> Result<(usize, usize, usize), String> {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let parsed = sqlquack_syntax_sqlite::parse(&sql).map_err(|error| error.message)?;
            let schema: TypedShopSchema = sqlite_shop_schema();
            let context = LineageContext {
                profile: &SQLITE,
                schema: &schema,
                sql: Some(&sql),
            };
            let statement: &sqlquack_syntax::Expression = &parsed.statements[0];
            Ok((
                tree_depth(statement),
                crate::statement_lineage(statement, context).outputs.len(),
                crate::statement_name_findings(statement, context).len(),
            ))
        })
        .unwrap_or_else(|error| panic!("{error}"))
        .join()
        .unwrap_or_else(|_| panic!("the analysis thread panicked"))
}

/// The queries resolved and expressions traced analysing `sql` on a 2 MiB thread.
pub(crate) fn sqlite_analysis_work(sql: String) -> usize {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let parsed = sqlquack_syntax_sqlite::parse(&sql).unwrap_or_else(|error| {
                panic!("{}", error.message);
            });
            let schema: TypedShopSchema = sqlite_shop_schema();
            let context = LineageContext {
                profile: &SQLITE,
                schema: &schema,
                sql: Some(&sql),
            };
            let statement: &sqlquack_syntax::Expression = &parsed.statements[0];
            crate::lineage::_helpers::tracing::reuse::work_done();
            crate::statement_lineage(statement, context);
            crate::statement_name_findings(statement, context);
            crate::lineage::_helpers::tracing::reuse::work_done()
        })
        .unwrap_or_else(|error| panic!("{error}"))
        .join()
        .unwrap_or_else(|_| panic!("the analysis thread panicked"))
}

/// A SQLite shop, with the declared types its tables are created with.
pub(crate) fn sqlite_shop_schema() -> TypedShopSchema {
    TypedShopSchema {
        closed: HashMap::from([
            (
                "orders".to_owned(),
                vec![
                    typed_column("id", "INTEGER"),
                    typed_column("quantity", "INTEGER"),
                    typed_column("price", "REAL"),
                    typed_column("status", "TEXT"),
                ],
            ),
            (
                "customers".to_owned(),
                vec![typed_column("id", "INTEGER"), typed_column("name", "TEXT")],
            ),
        ]),
        ..TypedShopSchema::default()
    }
}

/// A SELECT of `orders` wrapped in `depth` derived tables.
pub(crate) fn nested_derived_tables(depth: usize) -> String {
    let mut sql: String = "SELECT id FROM orders".to_owned();
    for _ in 0..depth {
        sql = format!("SELECT * FROM ({sql}) AS t");
    }
    sql
}

/// The number of levels in the tree under `root`, counted level by level without recursion.
pub(crate) fn tree_depth(root: &sqlquack_syntax::Expression) -> usize {
    use sqlquack_syntax::ExpressionWalk;
    std::iter::successors(Some(vec![root]), |level| {
        let next: Vec<&sqlquack_syntax::Expression> =
            level.iter().flat_map(|node| node.children()).collect();
        (!next.is_empty()).then_some(next)
    })
    .count()
}
