use std::collections::HashMap;

use crate::dialects::constants::DATABRICKS;
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::helpers::typed_column;
use crate::lineage::tests::test_types::TypedShopSchema;

/// `sql`'s one statement, read by the Databricks parser, refusing any gap.
fn databricks_statement(sql: &str) -> sqlquack_syntax::Expression {
    sqlquack_syntax_databricks::parse_strict(sql)
        .unwrap_or_else(|error| panic!("{sql}: {error:?}"))
        .statements
        .remove(0)
}

/// The `(code, message, start)` findings of Databricks `sql` under the DATABRICKS profile.
pub(crate) fn databricks_findings(sql: &str) -> Vec<(&'static str, String, usize)> {
    let schema: TypedShopSchema = databricks_shop_schema();
    let context = LineageContext {
        profile: &DATABRICKS,
        schema: &schema,
        sql: Some(sql),
    };
    statement_findings(&databricks_statement(sql), context)
        .into_iter()
        .map(|finding| {
            let start: usize = finding.span.map_or(0, |span| span.start);
            (finding.code, finding.message, start)
        })
        .collect()
}

/// Each output of Databricks `sql` under the DATABRICKS profile: its name and type.
pub(crate) fn databricks_outputs(sql: &str) -> Vec<(String, Option<String>)> {
    let schema: TypedShopSchema = databricks_shop_schema();
    let context = LineageContext {
        profile: &DATABRICKS,
        schema: &schema,
        sql: Some(sql),
    };
    statement_lineage(&databricks_statement(sql), context)
        .outputs
        .into_iter()
        .map(|output| (output.name, output.data_type))
        .collect()
}

/// A Databricks shop, typed as `DESCRIBE TABLE` spells its columns.
pub(crate) fn databricks_shop_schema() -> TypedShopSchema {
    TypedShopSchema {
        closed: HashMap::from([
            (
                "orders".to_owned(),
                vec![
                    typed_column("id", "INT"),
                    typed_column("quantity", "INT"),
                    typed_column("price", "DECIMAL(10,2)"),
                    typed_column("status", "STRING"),
                    typed_column("ordered_at", "TIMESTAMP"),
                ],
            ),
            (
                "customers".to_owned(),
                vec![typed_column("id", "INT"), typed_column("name", "STRING")],
            ),
        ]),
        ..TypedShopSchema::default()
    }
}
