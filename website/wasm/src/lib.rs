//! The playground's engine: one call runs SQLQuack's stages on some Snowflake SQL and reports each.
//!
//! Every stage is the existing crates' code. This crate only strings them together as
//! `sqlquack convert` does, and returns the result as JSON for the page.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlquack_core::classification::main::classify::classify;
use sqlquack_core::diagnostics::main::diagnose::diagnose;
use sqlquack_core::parsing::main::node_location::node_location;
use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::{SourceDialect, SourceLines};
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::main::refuse_constructs::refuse_constructs;
use sqlquack_core::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_core::refusals::main::snowflake_rejections::snowflake_rejections;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_duckdb::blocks::main::column_type::column_type;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_duckdb::checking::constants::SEMI_STRUCTURED_HOLDING_TYPE;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::main::convert_statement::convert_statement;
use sqlquack_rules::conversion::main::quoted_exclusions::quoted_exclusions;
use sqlquack_rules::conversion::models::ConversionOptions;
use sqlquack_semantics::dialects::constants::SNOWFLAKE;
use sqlquack_semantics::lineage::main::query_name_findings::query_name_findings;
use sqlquack_semantics::lineage::models::{LineageContext, Nullability, SchemaColumn};
use sqlquack_semantics::lineage::types::LineageSchema;
use sqlquack_semantics::types::models::{ColumnSpec, Schema};
use sqlquack_snowflake::typing::main::cast_target::cast_target;
use sqlquack_syntax::{Dialect, Expression, ParseOptions};
use wasm_bindgen::prelude::wasm_bindgen;

/// Every stage's result for `sql` over the relations `schema` declares, as JSON.
#[wasm_bindgen]
pub fn analyze(sql: &str, schema: &str) -> String {
    report(sql, schema).to_string()
}

/// `CREATE TABLE` for each relation in `schema`, in the DuckDB types converted SQL reads them as,
/// as JSON: `{"tables": [...]}`, or `{"error": ...}`.
#[wasm_bindgen]
pub fn fixture_tables(schema: &str) -> String {
    let catalog = match catalog(schema) {
        Ok(catalog) => catalog,
        Err(error) => return json!({ "error": error }).to_string(),
    };
    let log = ProvenanceLog::default();
    let tables: Vec<String> = catalog
        .schema
        .relations
        .iter()
        .map(|(relation, columns)| {
            let definitions: Vec<String> = columns
                .iter()
                .filter_map(|column| {
                    let held = match column_type(column.data_type, &log) {
                        Ok(held) => held,
                        Err(_) if column.data_type == sqlquack_snowflake::typing::models::SnowflakeType::SemiStructured => {
                            SEMI_STRUCTURED_HOLDING_TYPE.to_owned()
                        }
                        Err(_) => return None,
                    };
                    Some(format!("{} {held}", quoted_identifier(&column.name, &log)))
                })
                .collect();
            format!("CREATE TABLE {} ({})", quoted_identifier(relation, &log), definitions.join(", "))
        })
        .collect();
    json!({ "tables": tables }).to_string()
}

/// The relations of a schema file: `{"ORDERS": [["ORDER_ID", "NUMBER(38,0)"], …]}`, or the same
/// under `"relations"`. Columns may also be `{"name": …, "type": …}` objects.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawSchema {
    Sections {
        relations: BTreeMap<String, Vec<RawColumn>>,
    },
    Relations(BTreeMap<String, Vec<RawColumn>>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawColumn {
    Pair(String, String),
    Described {
        name: String,
        #[serde(rename = "type")]
        type_name: String,
    },
}

/// The schema's relations as conversion types them, and as name checks resolve them.
#[derive(Debug, Default)]
struct Catalog {
    schema: Schema,
    columns: BTreeMap<String, Vec<SchemaColumn>>,
}

impl LineageSchema for Catalog {
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        self.columns.get(relation).map(Vec::as_slice)
    }
}

fn catalog(text: &str) -> Result<Catalog, String> {
    if text.trim().is_empty() {
        return Ok(Catalog::default());
    }
    let relations = match serde_json::from_str(text).map_err(|error| error.to_string())? {
        RawSchema::Sections { relations } | RawSchema::Relations(relations) => relations,
    };
    let log = ProvenanceLog::default();
    let mut catalog = Catalog::default();
    for (relation, raw) in relations {
        let mut typed = Vec::with_capacity(raw.len());
        let mut named = Vec::with_capacity(raw.len());
        for column in raw {
            let (name, type_name) = match column {
                RawColumn::Pair(name, type_name) | RawColumn::Described { name, type_name } => {
                    (name, type_name)
                }
            };
            let target = cast_target(&type_name, &log)
                .map_err(|error| format!("{relation}.{name}: {error}"))?;
            named.push(SchemaColumn {
                name: name.clone(),
                quoted: false,
                data_type: Some(type_name),
                nullability: Nullability::Unknown,
            });
            typed.push(ColumnSpec::new(name, target.data_type));
        }
        catalog.columns.insert(relation.clone(), named);
        catalog.schema.relations.insert(relation, typed);
    }
    Ok(catalog)
}

/// A stage's verdict and what it found.
#[derive(Serialize)]
struct Stage {
    /// `pass`, `fail`, or `skipped` when an earlier stage failed.
    status: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<Value>,
    #[serde(flatten)]
    extra: BTreeMap<&'static str, Value>,
}

impl Stage {
    fn new(passed: bool, diagnostics: Vec<Value>) -> Self {
        Self {
            status: if passed { "pass" } else { "fail" },
            diagnostics,
            extra: BTreeMap::new(),
        }
    }

    fn skipped() -> Self {
        Self {
            status: "skipped",
            diagnostics: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

fn diagnostics(refusals: &[Refusal], sql: &str) -> Vec<Value> {
    diagnose(refusals, sql)
        .iter()
        .filter_map(|diagnostic| serde_json::to_value(diagnostic).ok())
        .collect()
}

/// The refusal at its own place, else at its statement, else over the whole source.
fn placed(mut refusal: Refusal, statement: &Expression, lines: &SourceLines) -> Refusal {
    refusal.location = refusal
        .location
        .or_else(|| node_location(statement, lines))
        .or_else(|| Some(lines.whole()));
    refusal
}

fn report(sql: &str, schema: &str) -> Value {
    let catalog = match catalog(schema) {
        Ok(catalog) => catalog,
        Err(error) => return json!({ "schema_error": error }),
    };
    let lines = SourceLines::new(sql);

    // 1. Parse as Snowflake.
    let statements = match parse_source(sql, SourceDialect::Snowflake) {
        Ok(statements) if statements.is_empty() => {
            return json!({ "empty": true });
        }
        Ok(statements) => statements,
        Err(failure) => {
            let parse = Stage::new(
                false,
                diagnostics(&[refuse_unparsed(&failure, &lines)], sql),
            );
            return json!({ "parse": parse, "checks": Stage::skipped(), "translate": Stage::skipped() });
        }
    };
    let mut parse = Stage::new(true, Vec::new());
    parse.extra.insert("statements", json!(statements.len()));

    // 2. What Snowflake rejects, then name and scope checks over the schema.
    let rejections: Vec<Refusal> = statements
        .iter()
        .flat_map(|statement| snowflake_rejections(statement, (sql, &lines)))
        .collect();
    let findings: Vec<Value> = match (catalog.columns.is_empty(), statements.len()) {
        (false, 1) => {
            let context = LineageContext {
                profile: &SNOWFLAKE,
                schema: &catalog,
                sql: Some(sql),
            };
            query_name_findings(sql, Dialect::Snowflake, &ParseOptions::default(), context)
                .unwrap_or_default()
                .into_iter()
                .map(|finding| {
                    json!({
                        "code": finding.code,
                        "message": finding.message,
                        "span": finding.span.map(|span| json!({ "start": span.start, "end": span.end })),
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    };
    let mut checks = Stage::new(
        rejections.is_empty() && findings.is_empty(),
        diagnostics(&rejections, sql),
    );
    checks.extra.insert("findings", json!(findings));
    checks.extra.insert(
        "names_checked",
        json!(!catalog.columns.is_empty() && statements.len() == 1),
    );
    if !rejections.is_empty() {
        return json!({ "parse": parse, "checks": checks, "translate": Stage::skipped() });
    }

    // 3. Convert each statement, or refuse it, as `sqlquack convert --builtin-policy` does.
    let policy = builtin_catalogue();
    let options = ConversionOptions {
        quoted_exclusions: quoted_exclusions(sql),
        ..ConversionOptions::default()
    };
    let mut refusals = Vec::new();
    let mut converted = Vec::new();
    for statement in &statements {
        let occurrences = classify(statement, &lines, SourceDialect::Snowflake);
        let declared: Vec<Refusal> = refuse_constructs(&occurrences, &policy)
            .into_iter()
            .filter(|refusal| refusal.category == RefusalCategory::NotExpressible)
            .collect();
        if !declared.is_empty() {
            refusals.extend(declared);
            continue;
        }
        match convert_statement(
            statement,
            (&catalog.schema, &options, &lines),
            ProvenanceLog::default(),
        ) {
            Ok(done) => converted.push(json!({
                "sql": done.sql,
                "columns": done.columns.iter().map(|column| json!({
                    "name": column.name,
                    "type": column.data_type.to_string(),
                })).collect::<Vec<_>>(),
            })),
            Err(refusal) => refusals.push(placed(refusal, statement, &lines)),
        }
    }
    let mut translate = Stage::new(refusals.is_empty(), diagnostics(&refusals, sql));
    translate.extra.insert("statements", json!(converted));
    json!({ "parse": parse, "checks": checks, "translate": translate })
}
