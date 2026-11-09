use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::DUCKDB;
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::helpers::duckdb_shop_schema;
use crate::lineage::tests::helpers::replayed_types;
use crate::lineage::tests::test_types::{EngineTypeReplay, TypedShopSchema};

/// Replays engine-recorded DuckDB result types: wrong types, exact count and per-category tally.
pub(crate) fn replayed_engine_types(fixture: &str) -> EngineTypeReplay {
    replayed_types(fixture, canonical_duckdb, duckdb_output_type)
}

/// The first output's type, when the query reads.
pub(crate) fn duckdb_output_type(schema: &TypedShopSchema, sql: &str) -> Option<String> {
    let context = LineageContext {
        profile: &DUCKDB,
        schema,
        sql: Some(sql),
    };
    let lineage = query_lineage(sql, Dialect::DuckDB, &ParseOptions::default(), context).ok()?;
    lineage.outputs.first()?.data_type.clone()
}

/// The findings of `sql` read by DuckDB's own parser, under the DUCKDB profile and its shop.
pub(crate) fn duckdb_parser_findings(sql: &str) -> Vec<(&'static str, String, usize)> {
    let statement = sqlquack_syntax_duckdb::parse_strict(sql)
        .unwrap_or_else(|failure| panic!("{sql}: {failure:?}"))
        .statements
        .remove(0);
    let schema = duckdb_shop_schema();
    let context = LineageContext {
        profile: &DUCKDB,
        schema: &schema,
        sql: Some(sql),
    };
    statement_findings(&statement, context)
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

/// A DuckDB type spelling in one canonical form, as the recorder's; VARCHAR(n) is VARCHAR.
pub(crate) fn canonical_duckdb(raw: &str) -> String {
    let upper: String = raw.trim().to_ascii_uppercase().replace(", ", ",");
    let base: &str = upper.split('(').next().unwrap_or_default().trim();
    let text: bool = CANONICAL_DUCKDB_TEXT.contains(&base);
    let spelled: &str = [base, upper.as_str()][usize::from(!text)];
    CANONICAL_DUCKDB_NAMES
        .iter()
        .find(|(spellings, _)| spellings.contains(&spelled))
        .map_or(spelled.to_owned(), |(_, name)| (*name).to_owned())
}

/// The DuckDB text spellings, whose length is not part of the type.
const CANONICAL_DUCKDB_TEXT: [&str; 6] = [
    "VARCHAR",
    "TEXT",
    "STRING",
    "CHAR",
    "BPCHAR",
    "CHARACTER VARYING",
];

/// The spellings of each DuckDB type and the one the comparison uses.
const CANONICAL_DUCKDB_NAMES: [(&[&str], &str); 19] = [
    (&["TINYINT", "INT1"], "TINYINT"),
    (&["SMALLINT", "INT2", "SHORT"], "SMALLINT"),
    (&["INTEGER", "INT", "INT4", "SIGNED"], "INTEGER"),
    (&["BIGINT", "INT8", "LONG"], "BIGINT"),
    (&["HUGEINT", "INT128"], "HUGEINT"),
    (&["UTINYINT", "UINT8"], "UTINYINT"),
    (&["USMALLINT", "UINT16"], "USMALLINT"),
    (&["UINTEGER", "UINT32"], "UINTEGER"),
    (&["UBIGINT", "UINT64"], "UBIGINT"),
    (&["UHUGEINT", "UINT128"], "UHUGEINT"),
    (&["FLOAT", "FLOAT4", "REAL"], "FLOAT"),
    (&["DOUBLE", "FLOAT8", "DOUBLE PRECISION"], "DOUBLE"),
    (
        &[
            "VARCHAR",
            "TEXT",
            "STRING",
            "CHAR",
            "BPCHAR",
            "CHARACTER VARYING",
        ],
        "VARCHAR",
    ),
    (&["BOOLEAN", "BOOL"], "BOOLEAN"),
    (
        &["TIMESTAMP", "DATETIME", "TIMESTAMP WITHOUT TIME ZONE"],
        "TIMESTAMP",
    ),
    (&["TIMESTAMPTZ", "TIMESTAMP WITH TIME ZONE"], "TIMESTAMPTZ"),
    (&["TIME", "TIME WITHOUT TIME ZONE"], "TIME"),
    (&["TIMETZ", "TIME WITH TIME ZONE"], "TIMETZ"),
    (&["BLOB", "BYTEA", "VARBINARY"], "BLOB"),
];
