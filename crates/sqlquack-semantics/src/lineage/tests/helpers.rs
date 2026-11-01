use std::collections::HashMap;

use crate::lineage::models::NullabilityRule;
use crate::lineage::models::{Nullability, SchemaColumn, SourceColumn};
use sqlquack_syntax::{CallMode, Dialect, ParseOptions, parse_statements_with};

use crate::dialects::constants::DUCKDB;
use crate::dialects::types::DialectProfile;
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::models::LineageContext;
use crate::lineage::tests::test_types::{
    EngineTypeFixture, EngineTypeReplay, EngineTypeTally, ShopSchema, TypedShopSchema,
};
use crate::lineage::types::LineageSchema;

impl LineageSchema for ShopSchema {
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        self.0.get(relation).map(Vec::as_slice)
    }
}

/// Orders, customers and products, as a Snowflake catalog spells unquoted names.
pub(crate) fn shop_schema() -> ShopSchema {
    schema_of([
        (
            "ORDERS",
            &["ID", "CUSTOMER_ID", "AMOUNT", "ITEMS", "STATUS"],
        ),
        ("CUSTOMERS", &["ID", "NAME", "COUNTRY"]),
        ("SHOP.PRODUCTS", &["SKU", "PRICE"]),
        ("EXPORTS", &["\"listingId\"", "LISTING_STATUS"]),
    ])
}

/// The same shop as DuckDB spells unquoted names, in lower case.
pub(crate) fn duckdb_shop_schema() -> ShopSchema {
    schema_of([
        (
            "orders",
            &["id", "customer_id", "amount", "items", "status"],
        ),
        ("customers", &["id", "name", "country"]),
        ("shop.products", &["sku", "price"]),
        ("exports", &["\"listingId\"", "listing_status"]),
    ])
}

/// Relations of untyped columns, a quoted column written in its quotes.
fn schema_of(relations: [(&str, &[&str]); 4]) -> ShopSchema {
    ShopSchema(
        relations
            .into_iter()
            .map(|(name, columns)| {
                (
                    name.to_owned(),
                    columns
                        .iter()
                        .map(|column| SchemaColumn {
                            name: column.trim_matches('"').to_owned(),
                            quoted: column.starts_with('"'),
                            data_type: None,
                            nullability: Nullability::Unknown,
                        })
                        .collect(),
                )
            })
            .collect(),
    )
}

/// The `(code, message, start)` findings of `sql`, parsed and read in one dialect.
pub(crate) fn found_findings(
    sql: &str,
    (dialect, profile, schema): (Dialect, &'static dyn DialectProfile, &dyn LineageSchema),
) -> Vec<(&'static str, String, usize)> {
    let context = LineageContext {
        profile,
        schema,
        sql: Some(sql),
    };
    let statements = parse_statements_with(
        sql,
        dialect,
        &ParseOptions::default(),
        CallMode::FaithfulLocated,
    )
    .unwrap_or_else(|error| panic!("{sql}: {error}"));
    statement_findings(&statements[0], context)
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

/// `expected` as owned `(code, message, start)` findings.
pub(crate) fn owned_findings(
    expected: &[(&'static str, &str, usize)],
) -> Vec<(&'static str, String, usize)> {
    expected
        .iter()
        .map(|(code, message, start)| (*code, (*message).to_owned(), *start))
        .collect()
}

/// A source column as `RELATION.column`, a quoted column in quotes.
pub(crate) fn written(source: &SourceColumn) -> String {
    let quote: &str = ["", "\""][usize::from(source.quoted)];
    format!("{}.{quote}{}{quote}", source.relation, source.column)
}

impl LineageSchema for TypedShopSchema {
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        self.closed.get(relation).map(Vec::as_slice)
    }

    fn open_columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        self.open.get(relation).map(Vec::as_slice)
    }

    fn function_type(&self, name: &str) -> Option<&str> {
        self.functions.get(&name.to_uppercase()).map(String::as_str)
    }

    fn function_nullability(&self, name: &str) -> Option<NullabilityRule> {
        self.rules.get(&name.to_uppercase()).copied()
    }

    fn function_arities(&self, name: &str, canonical: &str) -> Option<Vec<(usize, Option<usize>)>> {
        let _ = name;
        self.arities.get(canonical).cloned()
    }

    fn known_function(&self, name: &str) -> bool {
        self.declared
            .iter()
            .any(|declared| declared.eq_ignore_ascii_case(name))
    }
}

/// Orders and customers with types and nullability, an open events relation, and two functions.
pub(crate) fn typed_shop_schema() -> TypedShopSchema {
    let column = |name: &str, data_type: &str, nullability: Nullability| SchemaColumn {
        name: name.to_owned(),
        quoted: false,
        data_type: Some(data_type.to_owned()),
        nullability,
    };
    TypedShopSchema {
        closed: HashMap::from([
            (
                "ORDERS".to_owned(),
                vec![
                    column("ID", "NUMBER(38,0)", Nullability::NonNull),
                    column("CUSTOMER_ID", "NUMBER(38,0)", Nullability::Unknown),
                    column("AMOUNT", "NUMBER(10,2)", Nullability::Nullable),
                    column("STATUS", "VARCHAR", Nullability::Unknown),
                ],
            ),
            (
                "CUSTOMERS".to_owned(),
                vec![
                    column("ID", "NUMBER(38,0)", Nullability::NonNull),
                    column("NAME", "VARCHAR(100)", Nullability::NonNull),
                ],
            ),
        ]),
        open: HashMap::from([(
            "EVENTS".to_owned(),
            vec![column("PAYLOAD", "VARIANT", Nullability::NonNull)],
        )]),
        functions: HashMap::from([
            ("TO_JSON".to_owned(), "TEXT".to_owned()),
            ("LISTAGG".to_owned(), "TEXT".to_owned()),
        ]),
        rules: HashMap::from([
            ("IFF".to_owned(), NullabilityRule::ConditionalResult),
            ("UPPER".to_owned(), NullabilityRule::FirstArgument),
        ]),
        declared: Vec::new(),
        arities: HashMap::new(),
    }
}

/// A DuckDB shop of typed columns: orders with numbers, text and a timestamp.
pub(crate) fn typed_duckdb_shop_schema() -> TypedShopSchema {
    TypedShopSchema {
        closed: HashMap::from([
            (
                "orders".to_owned(),
                vec![
                    typed_column("id", "INTEGER"),
                    typed_column("quantity", "INTEGER"),
                    typed_column("price", "DECIMAL(10,2)"),
                    typed_column("status", "VARCHAR"),
                    typed_column("ordered_at", "TIMESTAMP"),
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
        declared: vec!["shop_score".to_owned()],
        arities: HashMap::from([
            ("substring".to_owned(), vec![(2, Some(3))]),
            ("coalesce".to_owned(), vec![(1, None)]),
        ]),
        ..TypedShopSchema::default()
    }
}

pub(crate) fn typed_column(name: &str, data_type: &str) -> SchemaColumn {
    SchemaColumn {
        name: name.to_owned(),
        quoted: false,
        data_type: Some(data_type.to_owned()),
        nullability: Nullability::Unknown,
    }
}

fn engine_schema(fixture: &EngineTypeFixture) -> TypedShopSchema {
    TypedShopSchema {
        closed: fixture
            .schema
            .tables
            .iter()
            .map(|table| {
                (
                    table.name.clone(),
                    table
                        .columns
                        .iter()
                        .map(|column| typed_column(&column.name, &column.data_type))
                        .collect(),
                )
            })
            .collect(),
        open: HashMap::new(),
        functions: HashMap::new(),
        rules: HashMap::new(),
        declared: Vec::new(),
        arities: HashMap::new(),
    }
}

/// Replays engine-recorded DuckDB result types: wrong types, exact count and per-category tally.
pub(crate) fn replayed_engine_types(fixture: &str) -> EngineTypeReplay {
    let fixture: EngineTypeFixture =
        serde_json::from_str(fixture).unwrap_or_else(|error| panic!("fixture: {error}"));
    let schema: TypedShopSchema = engine_schema(&fixture);
    let mut replay = EngineTypeReplay::default();
    let recorded = fixture.cases.iter().filter_map(|case| {
        let engine_type: String = canonical_duckdb(case.engine_type.as_deref()?);
        let reported: Option<String> =
            duckdb_output_type(&schema, &case.sql).map(|reported| canonical_duckdb(&reported));
        Some((case, engine_type, reported))
    });
    for (case, engine_type, reported) in recorded {
        let exact: bool = reported.as_deref() == Some(engine_type.as_str());
        let counts: &mut EngineTypeTally = replay.tally.entry(case.category.clone()).or_default();
        counts.exact += usize::from(exact);
        counts.untyped += usize::from(reported.is_none());
        replay.typed += usize::from(exact);
        replay.wrong.extend(
            reported
                .filter(|_| !exact)
                .map(|reported| format!("{}: {reported} != {engine_type}", case.id)),
        );
    }
    replay
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

/// A DuckDB type spelling in one canonical form, as the recorder's; VARCHAR(n) is VARCHAR.
fn canonical_duckdb(raw: &str) -> String {
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
