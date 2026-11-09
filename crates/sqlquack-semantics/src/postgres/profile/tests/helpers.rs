use sqlquack_syntax::{Dialect, ParseOptions};

use crate::dialects::constants::POSTGRES;
use crate::dialects::types::DialectProfile;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::models::TransformKind;
use crate::lineage::tests::helpers::replayed_types;
use crate::lineage::tests::helpers::typed_column;
use crate::lineage::tests::helpers::typed_duckdb_shop_schema;
use crate::lineage::tests::test_types::{EngineTypeReplay, TypedShopSchema};
use crate::lineage::types::LineageSchema;

/// Each recorded PostgreSQL result type against the type SQLQuack reports with Engine::Postgres.
pub(crate) fn replayed_postgres_engine_types(fixture: &str) -> EngineTypeReplay {
    replayed_types(fixture, canonical_postgres, postgres_output_type)
}

/// Each recorded PostgreSQL result type against the type of the native PostgreSQL parser's tree.
pub(crate) fn replayed_native_postgres_engine_types(fixture: &str) -> EngineTypeReplay {
    replayed_types(fixture, canonical_postgres, native_postgres_output_type)
}

/// The first output's type under Engine::Postgres, when the query reads.
fn postgres_output_type(schema: &TypedShopSchema, sql: &str) -> Option<String> {
    let context = LineageContext {
        profile: &POSTGRES,
        schema,
        sql: Some(sql),
    };
    let lineage =
        query_lineage(sql, Dialect::PostgreSQL, &ParseOptions::default(), context).ok()?;
    lineage.outputs.first()?.data_type.clone()
}

/// The first output's type under Engine::Postgres, when the native parser reads one statement whole.
pub(crate) fn native_postgres_output_type(schema: &TypedShopSchema, sql: &str) -> Option<String> {
    let parsed = sqlquack_syntax_postgres::parse_strict(sql)
        .ok()
        .filter(|parsed| parsed.gaps.is_empty())?;
    let [statement] = parsed.statements.as_slice() else {
        return None;
    };
    let context = LineageContext {
        profile: &POSTGRES,
        schema,
        sql: Some(sql),
    };
    statement_lineage(statement, context)
        .outputs
        .first()?
        .data_type
        .clone()
}

/// Each output of `sql`'s lineage: its name, transform and sources as `relation.column`.
pub(crate) fn lineage_outputs(
    sql: &str,
    (dialect, profile): (Dialect, &'static dyn DialectProfile),
    schema: &dyn LineageSchema,
) -> Vec<(String, TransformKind, Vec<String>)> {
    let context = LineageContext {
        profile,
        schema,
        sql: None,
    };
    query_lineage(sql, dialect, &ParseOptions::default(), context)
        .unwrap_or_else(|error| panic!("{sql}: {error}"))
        .outputs
        .iter()
        .map(|output| {
            let sources: Vec<String> = output
                .sources
                .iter()
                .map(|source| format!("{}.{}", source.relation, source.column))
                .collect();
            (output.name.clone(), output.kind, sources)
        })
        .collect()
}

/// A PostgreSQL shop: the DuckDB shop's orders and customers, and shipments of temporal values.
pub(crate) fn typed_postgres_shop_schema() -> TypedShopSchema {
    let mut schema: TypedShopSchema = typed_duckdb_shop_schema();
    schema.closed.insert(
        "shipments".to_owned(),
        vec![
            typed_column("shipped_on", "DATE"),
            typed_column("shipped_at", "TIMESTAMP"),
            typed_column("cutoff", "TIME"),
            typed_column("took", "INTERVAL"),
            typed_column("rush", "BOOLEAN"),
        ],
    );
    schema
}

/// A PostgreSQL type spelling in one canonical form, keeping its modifiers as `format_type` does.
pub(crate) fn canonical_postgres(raw: &str) -> String {
    let upper: String = raw.trim().to_ascii_uppercase().replace(", ", ",");
    upper.strip_suffix("[]").map_or_else(
        || {
            let (head, rest) = upper.split_once('(').unwrap_or((upper.as_str(), ""));
            let (modifiers, suffix) = rest.split_once(')').unwrap_or_default();
            let base: String = format!("{} {}", head.trim(), suffix.trim())
                .trim()
                .to_owned();
            let name: String = CANONICAL_POSTGRES_NAMES
                .iter()
                .find(|(spellings, _)| spellings.contains(&base.as_str()))
                .map_or(base.clone(), |(_, name)| (*name).to_owned());
            let scale_omitted: bool =
                name == "NUMERIC" && !modifiers.is_empty() && !modifiers.contains(',');
            let modifiers: String = [modifiers.to_owned(), format!("{modifiers},0")]
                [usize::from(scale_omitted)]
            .clone();
            [name.clone(), format!("{name}({modifiers})")][usize::from(!modifiers.is_empty())]
                .clone()
        },
        |element| format!("{}[]", canonical_postgres(element)),
    )
}

/// The spellings of each PostgreSQL type and the one the comparison uses.
const CANONICAL_POSTGRES_NAMES: [(&[&str], &str); 14] = [
    (&["SMALLINT", "INT2"], "SMALLINT"),
    (&["INTEGER", "INT", "INT4"], "INTEGER"),
    (&["BIGINT", "INT8"], "BIGINT"),
    (&["NUMERIC", "DECIMAL"], "NUMERIC"),
    (&["REAL", "FLOAT4"], "REAL"),
    (
        &["DOUBLE PRECISION", "DOUBLE", "FLOAT8", "FLOAT"],
        "DOUBLE PRECISION",
    ),
    (&["VARCHAR", "CHARACTER VARYING"], "VARCHAR"),
    (&["CHAR", "CHARACTER", "BPCHAR"], "CHAR"),
    (&["BOOLEAN", "BOOL"], "BOOLEAN"),
    (&["TIMESTAMP", "TIMESTAMP WITHOUT TIME ZONE"], "TIMESTAMP"),
    (&["TIMESTAMPTZ", "TIMESTAMP WITH TIME ZONE"], "TIMESTAMPTZ"),
    (&["TIME", "TIME WITHOUT TIME ZONE"], "TIME"),
    (&["TIMETZ", "TIME WITH TIME ZONE"], "TIMETZ"),
    (&["TEXT"], "TEXT"),
];
