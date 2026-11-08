use std::collections::HashMap;

use crate::lineage::models::NullabilityRule;
use crate::lineage::models::{Nullability, SchemaColumn, SourceColumn};
use sqlquack_syntax::{CallMode, Dialect, Expression, ParseOptions, parse_statements_with};

use crate::dialects::constants::{
    BIGQUERY, CLICKHOUSE, DATABRICKS, DUCKDB, POSTGRES, SQL_SERVER, SQLITE,
};
use crate::dialects::types::DialectProfile;
use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::main::query_lineage::query_lineage;
use crate::lineage::main::statement_lineage::statement_lineage;
use crate::lineage::models::{LineageContext, TransformKind};
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
            (
                "SHIPMENTS".to_owned(),
                vec![
                    column("SHIPPED_ON", "DATE", Nullability::NonNull),
                    column("SHIPPED_AT", "TIMESTAMP_NTZ(9)", Nullability::NonNull),
                    column("CUTOFF", "TIME(9)", Nullability::NonNull),
                    column("PICKUP_TIME", "TIME(3)", Nullability::NonNull),
                    column("PACKED_AT", "TIMESTAMP_NTZ(3)", Nullability::NonNull),
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
    replayed_types(fixture, canonical_duckdb, duckdb_output_type)
}

/// Each recorded ClickHouse result type against the ClickHouse profile's, `Nullable` set aside.
pub(crate) fn replayed_clickhouse_engine_types(fixture: &str) -> EngineTypeReplay {
    replayed_types(fixture, unwrapped_clickhouse, clickhouse_output_type)
}

/// A ClickHouse type without its top-level `LowCardinality` and `Nullable` wrappers.
fn unwrapped_clickhouse(spelling: &str) -> String {
    ["LowCardinality(", "Nullable("]
        .iter()
        .fold(spelling.trim(), |text, wrapper| {
            text.strip_prefix(wrapper)
                .and_then(|inner| inner.strip_suffix(')'))
                .unwrap_or(text)
        })
        .to_owned()
}

/// The first output's type under the ClickHouse profile, when the native parser reads the query.
fn clickhouse_output_type(schema: &TypedShopSchema, sql: &str) -> Option<String> {
    let parsed = sqlquack_syntax_clickhouse::parse(sql).ok()?;
    let context = LineageContext {
        profile: &CLICKHOUSE,
        schema,
        sql: Some(sql),
    };
    statement_lineage(parsed.statements.first()?, context)
        .outputs
        .first()?
        .data_type
        .clone()
}

/// Each recorded PostgreSQL result type against the type SQLQuack reports with Engine::Postgres.
pub(crate) fn replayed_postgres_engine_types(fixture: &str) -> EngineTypeReplay {
    replayed_types(fixture, canonical_postgres, postgres_output_type)
}

/// Each recorded PostgreSQL result type against the type of the native PostgreSQL parser's tree.
pub(crate) fn replayed_native_postgres_engine_types(fixture: &str) -> EngineTypeReplay {
    replayed_types(fixture, canonical_postgres, native_postgres_output_type)
}

/// Each recorded engine type, in canonical form, against the type the lineage reports.
fn replayed_types(
    fixture: &str,
    canonical: fn(&str) -> String,
    output_type: fn(&TypedShopSchema, &str) -> Option<String>,
) -> EngineTypeReplay {
    let fixture: EngineTypeFixture =
        serde_json::from_str(fixture).unwrap_or_else(|error| panic!("fixture: {error}"));
    let schema: TypedShopSchema = engine_schema(&fixture);
    let mut replay = EngineTypeReplay::default();
    let recorded = fixture.cases.iter().filter_map(|case| {
        let engine_type: String = canonical(case.engine_type.as_deref()?);
        let reported: Option<String> =
            output_type(&schema, &case.sql).map(|reported| canonical(&reported));
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

/// A ClickHouse shop: types as ClickHouse names them.
pub(crate) fn clickhouse_shop_schema() -> TypedShopSchema {
    let table = |columns: &[(&str, &str)]| {
        columns
            .iter()
            .map(|(name, data_type)| typed_column(name, data_type))
            .collect()
    };
    TypedShopSchema {
        closed: HashMap::from([
            (
                "orders".to_owned(),
                table(&[
                    ("id", "Int32"),
                    ("quantity", "Int32"),
                    ("price", "Decimal(10, 2)"),
                    ("status", "String"),
                ]),
            ),
            (
                "customers".to_owned(),
                table(&[("id", "Int32"), ("name", "String")]),
            ),
            (
                "nested".to_owned(),
                table(&[
                    ("arr", "Array(Tuple(a Int32))"),
                    ("tup", "Tuple(a Int32)"),
                    ("zq.order_id", "Int32"),
                ]),
            ),
        ]),
        ..TypedShopSchema::default()
    }
}

/// Three ClickHouse tables: `a` and `b` share `id` and `v`, whose types differ; `c` shares none.
pub(crate) fn clickhouse_join_schema() -> TypedShopSchema {
    let table = |columns: &[(&str, &str)]| {
        columns
            .iter()
            .map(|(name, data_type)| typed_column(name, data_type))
            .collect()
    };
    TypedShopSchema {
        closed: HashMap::from([
            (
                "a".to_owned(),
                table(&[("id", "Int32"), ("v", "Int32"), ("k", "String")]),
            ),
            (
                "b".to_owned(),
                table(&[("id", "Int32"), ("v", "String"), ("w", "Int64")]),
            ),
            ("c".to_owned(), table(&[("z", "Int32")])),
        ]),
        ..TypedShopSchema::default()
    }
}

/// The first statement of `sql`, parsed by the native ClickHouse parser.
pub(crate) fn parsed_clickhouse(sql: &str) -> Expression {
    let mut parsed =
        sqlquack_syntax_clickhouse::parse(sql).unwrap_or_else(|error| panic!("{sql}: {error:?}"));
    parsed.statements.remove(0)
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

/// A SELECT of `orders` wrapped in `depth` derived tables.
pub(crate) fn nested_derived_tables(depth: usize) -> String {
    let mut sql: String = "SELECT id FROM orders".to_owned();
    for _ in 0..depth {
        sql = format!("SELECT * FROM ({sql}) AS t");
    }
    sql
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

/// The number of levels in the tree under `root`, counted level by level without recursion.
fn tree_depth(root: &sqlquack_syntax::Expression) -> usize {
    use sqlquack_syntax::ExpressionWalk;
    std::iter::successors(Some(vec![root]), |level| {
        let next: Vec<&sqlquack_syntax::Expression> =
            level.iter().flat_map(|node| node.children()).collect();
        (!next.is_empty()).then_some(next)
    })
    .count()
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
