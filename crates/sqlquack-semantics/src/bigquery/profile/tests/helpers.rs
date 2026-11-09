use std::collections::HashMap;

use crate::dialects::constants::BIGQUERY;
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::helpers::typed_column;
use crate::lineage::tests::test_types::TypedShopSchema;
use crate::lineage::types::LineageSchema;

/// The `(code, message, start)` findings of `sql`, parsed by the BigQuery parser.
pub(crate) fn bigquery_findings(
    sql: &str,
    schema: &dyn LineageSchema,
) -> Vec<(&'static str, String, usize)> {
    let context = LineageContext {
        profile: &BIGQUERY,
        schema,
        sql: Some(sql),
    };
    let parsed =
        sqlquack_syntax_bigquery::parse(sql).unwrap_or_else(|error| panic!("{sql}: {error:?}"));
    statement_findings(&parsed.statements[0], context)
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

/// The first output's type under the BigQuery profile.
pub(crate) fn bigquery_output_type(schema: &TypedShopSchema, sql: &str) -> Option<String> {
    let context = LineageContext {
        profile: &BIGQUERY,
        schema,
        sql: Some(sql),
    };
    let parsed = sqlquack_syntax_bigquery::parse(sql).ok()?;
    statement_lineage(parsed.statements.first()?, context)
        .outputs
        .first()?
        .data_type
        .clone()
}

/// A BigQuery shop: orders of each scalar type, and a customers column in its catalog's case.
pub(crate) fn typed_bigquery_shop_schema() -> TypedShopSchema {
    TypedShopSchema {
        closed: HashMap::from([
            (
                "orders".to_owned(),
                vec![
                    typed_column("id", "INT64"),
                    typed_column("customer_id", "INT64"),
                    typed_column("quantity", "INT64"),
                    typed_column("price", "NUMERIC"),
                    typed_column("ratio", "FLOAT64"),
                    typed_column("status", "STRING"),
                    typed_column("ordered_at", "TIMESTAMP"),
                    typed_column("ordered_on", "DATE"),
                    typed_column("rush", "BOOL"),
                ],
            ),
            (
                "customers".to_owned(),
                vec![
                    typed_column("id", "INT64"),
                    typed_column("FullName", "STRING"),
                ],
            ),
        ]),
        ..TypedShopSchema::default()
    }
}
