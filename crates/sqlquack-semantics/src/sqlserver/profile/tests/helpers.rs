use std::collections::HashMap;

use crate::dialects::constants::SQL_SERVER;
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::helpers::typed_column;
use crate::lineage::tests::test_types::TypedShopSchema;

/// The one statement SQL Server's parser reads from `sql`.
fn sqlserver_statement(sql: &str) -> sqlquack_syntax::Expression {
    let parsed = sqlquack_syntax_sqlserver::parse_strict(sql)
        .unwrap_or_else(|failure| panic!("{sql}: {failure:?}"));
    assert!(parsed.gaps.is_empty(), "{sql}: {:?}", parsed.gaps);
    parsed
        .statements
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{sql}: no statement"))
}

/// Each SQL Server output's name and type.
pub(crate) fn sqlserver_outputs(
    schema: &TypedShopSchema,
    sql: &str,
) -> Vec<(String, Option<String>)> {
    let context = LineageContext {
        profile: &SQL_SERVER,
        schema,
        sql: Some(sql),
    };
    statement_lineage(&sqlserver_statement(sql), context)
        .outputs
        .into_iter()
        .map(|output| (output.name, output.data_type))
        .collect()
}

/// SQL Server's findings for `sql` as `(code, message, start)`.
pub(crate) fn sqlserver_findings(
    schema: &TypedShopSchema,
    sql: &str,
) -> Vec<(&'static str, String, usize)> {
    let context = LineageContext {
        profile: &SQL_SERVER,
        schema,
        sql: Some(sql),
    };
    statement_findings(&sqlserver_statement(sql), context)
        .into_iter()
        .map(|finding| {
            (
                finding.code,
                finding.message,
                finding.span.map_or(0, |span| span.start),
            )
        })
        .collect()
}

/// A SQL Server shop: orders with SQLBuild's catalog spellings, TIMESTAMP and DOUBLE included.
pub(crate) fn sqlserver_shop_schema() -> TypedShopSchema {
    TypedShopSchema {
        closed: HashMap::from([
            (
                "orders".to_owned(),
                vec![
                    typed_column("id", "INTEGER"),
                    typed_column("big", "BIGINT"),
                    typed_column("amount", "DOUBLE"),
                    typed_column("price", "DECIMAL(12, 2)"),
                    typed_column("status", "VARCHAR"),
                    typed_column("created_at", "DATETIME2"),
                    typed_column("logged_at", "TIMESTAMP"),
                    typed_column("label", "NVARCHAR(40)"),
                ],
            ),
            (
                "customers".to_owned(),
                vec![
                    typed_column("id", "INTEGER"),
                    typed_column("name", "VARCHAR"),
                ],
            ),
        ]),
        ..TypedShopSchema::default()
    }
}
