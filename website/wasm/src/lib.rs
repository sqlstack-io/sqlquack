//! The playground's engine: one call runs SQLQuack's stages on some Snowflake SQL and reports each.
//!
//! Every stage is the existing crates' code. This crate only strings them together as
//! `sqlquack convert` does, and returns the result as JSON for the page.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlquack_core::classification::main::classify::classify;
use sqlquack_core::diagnostics::constants::SESSION_PARAMETERS;
use sqlquack_core::diagnostics::main::diagnose::diagnose;
use sqlquack_core::diagnostics::main::unplaced_diagnostic::unplaced_diagnostic;
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
use sqlquack_format::pretty_sql;
use sqlquack_policy::policy::models::Policy;
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::main::convert_statement::convert_statement;
use sqlquack_rules::conversion::main::quoted_exclusions::quoted_exclusions;
use sqlquack_rules::conversion::main::within_budget::within_budget;
use sqlquack_rules::conversion::models::ConversionOptions;
use sqlquack_rules::session::main::parse_session::parse_session;
use sqlquack_rules::session::main::session_conversion_options::session_conversion_options;
use sqlquack_semantics::dialects::constants::SNOWFLAKE;
use sqlquack_semantics::lineage::main::query_name_findings::query_name_findings;
use sqlquack_semantics::lineage::models::{LineageContext, SchemaCatalog};
use sqlquack_semantics::types::main::parse_schema::parse_schema;
use sqlquack_semantics::types::models::Schema;
use sqlquack_syntax::{Dialect, Expression, ParseOptions};
use wasm_bindgen::prelude::wasm_bindgen;

/// Every stage's result for `sql` over the relations `schema` declares, as JSON. `settings` is
/// JSON `{"policy": TOML, "session": "NAME=VALUE" lines, "strict": bool, "utc_session": bool}`;
/// an empty policy is the built-in default, as `sqlquack convert --builtin-policy`.
#[wasm_bindgen]
pub fn analyze(sql: &str, schema: &str, settings: Option<String>) -> String {
    report(sql, schema, settings.as_deref().unwrap_or_default()).to_string()
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

/// The name check's code for a column no FROM item offers.
const UNKNOWN_COLUMN_FINDING: &str = "E201";

/// The schema's relations as conversion types them, and as name checks resolve them.
#[derive(Debug, Default)]
struct Catalog {
    schema: Schema,
    names: SchemaCatalog,
}

/// The schema file `text`, read by the same parser as `sqlquack convert --schema`.
fn catalog(text: &str) -> Result<Catalog, String> {
    if text.trim().is_empty() {
        return Ok(Catalog::default());
    }
    let schema = parse_schema(text).map_err(|error| error.to_string())?;
    let names = SchemaCatalog::new(&schema);
    Ok(Catalog { schema, names })
}

/// The playground's policy and session settings.
#[derive(Default, Deserialize)]
#[serde(default)]
struct Settings {
    policy: String,
    session: String,
    strict: bool,
    utc_session: bool,
}

/// Conversion options for `settings` as `sqlquack convert` chooses them, and the matched profile;
/// a session no recorded profile covers is a refusal, as on the command line.
fn chosen_options(settings: &str) -> Result<Result<(ConversionOptions, String), String>, String> {
    let settings: Settings = match settings.trim() {
        "" => Settings::default(),
        text => serde_json::from_str(text).map_err(|error| error.to_string())?,
    };
    let policy = match settings.policy.trim() {
        "" => None,
        text => Some(
            text.parse::<Policy>()
                .map_err(|error| format!("policy: {error}"))?,
        ),
    };
    let lines: Vec<String> = settings
        .session
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    let parameters = parse_session(&lines)?;
    Ok(session_conversion_options(
        policy.as_ref(),
        (settings.strict, settings.utc_session),
        &parameters,
    ))
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

fn report(sql: &str, schema: &str, settings: &str) -> Value {
    let catalog = match catalog(schema) {
        Ok(catalog) => catalog,
        Err(error) => return json!({ "schema_error": error }),
    };
    let session = match chosen_options(settings) {
        Ok(session) => session,
        Err(error) => return json!({ "settings_error": error }),
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
    let names_checked = !catalog.schema.relations.is_empty() && statements.len() == 1;
    let findings: Vec<Value> = match names_checked {
        true => {
            let context = LineageContext {
                profile: &SNOWFLAKE,
                schema: &catalog.names,
                sql: Some(sql),
            };
            query_name_findings(sql, Dialect::Snowflake, &ParseOptions::default(), context)
                .unwrap_or_default()
                .into_iter()
                .map(|finding| {
                    let written: Option<String> = finding
                        .span
                        .map(|span| sql.chars().skip(span.start).take(span.end - span.start).collect());
                    let help = written
                        .as_deref()
                        .and_then(|written| written.strip_prefix('"')?.strip_suffix('"'))
                        .filter(|_| finding.code == UNKNOWN_COLUMN_FINDING)
                        .map(|value| format!("use single quotes for a string literal: '{}'", value.replace('\'', "''")));
                    json!({
                        "code": finding.code,
                        "message": finding.message,
                        "span": finding.span.map(|span| json!({ "start": span.start, "end": span.end })),
                        "help": help,
                    })
                })
                .collect()
        }
        false => Vec::new(),
    };
    let mut checks = Stage::new(
        rejections.is_empty() && findings.is_empty(),
        diagnostics(&rejections, sql),
    );
    checks.extra.insert("findings", json!(findings));
    checks.extra.insert("names_checked", json!(names_checked));
    // SQL Snowflake rejects is not translated, as the native check would refuse it at DuckDB's
    // bind, which the browser does not run. Warnings (W codes) do not stop translation.
    let erred = findings.iter().any(|finding| {
        finding["code"]
            .as_str()
            .is_some_and(|code| code.starts_with('E'))
    });
    if !rejections.is_empty() || erred {
        return json!({ "parse": parse, "checks": checks, "translate": Stage::skipped() });
    }

    // 3. Convert each statement, or refuse it, as `sqlquack convert` does under the same policy.
    let policy = builtin_catalogue();
    let (options, profile) = match session {
        Ok(chosen) => chosen,
        Err(reason) => {
            let refused = unplaced_diagnostic(SESSION_PARAMETERS, reason)
                .and_then(|diagnostic| serde_json::to_value(diagnostic).ok());
            let translate = Stage::new(false, refused.into_iter().collect());
            return json!({ "parse": parse, "checks": checks, "translate": translate });
        }
    };
    let options = ConversionOptions {
        quoted_exclusions: quoted_exclusions(sql),
        ..options
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
        let started = now_ms();
        let done = convert_statement(
            statement,
            (&catalog.schema, &options, &lines),
            ProvenanceLog::default(),
        );
        let elapsed = Duration::from_secs_f64((now_ms() - started).max(0.0) / 1000.0);
        match done.and_then(|done| within_budget(done, (sql, elapsed), &options)) {
            Ok(done) => converted.push(json!({
                "pretty": pretty_sql(&done.sql, Dialect::DuckDB),
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
    translate.extra.insert("profile", json!(profile));
    json!({ "parse": parse, "checks": checks, "translate": translate })
}

/// Milliseconds on a monotonic clock: `performance.now()` in the browser, where `Instant` panics.
#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = performance, js_name = now)]
        fn performance_now() -> f64;
    }
    performance_now()
}

/// Milliseconds on a monotonic clock, for native builds of this crate.
#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}
