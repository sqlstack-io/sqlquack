//! The playground's engine: one call runs SQLQuack's stages on some Snowflake SQL and reports each.
//!
//! Every stage is the existing crates' code. This crate only strings them together as
//! `sqlquack convert` does, and returns the result as JSON for the page.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlquack_blocks::blocks::main::column_type::column_type;
use sqlquack_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_blocks::checking::constants::SEMI_STRUCTURED_HOLDING_TYPE;
use sqlquack_blocks::layout::main::layout_sql::layout_sql;
use sqlquack_diagnostics::classification::main::classify::classify;
use sqlquack_diagnostics::classification::models::SourceDialect;
use sqlquack_diagnostics::diagnostics::constants::SESSION_PARAMETERS;
use sqlquack_diagnostics::diagnostics::main::diagnose::diagnose;
use sqlquack_diagnostics::diagnostics::main::unplaced_diagnostic::unplaced_diagnostic;
use sqlquack_diagnostics::failures::constants::{ERROR_TAG_PREFIX, TAG_END};
use sqlquack_diagnostics::failures::main::duckdb_failure::duckdb_failure;
use sqlquack_diagnostics::failures::main::raise_id::raise_id;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_diagnostics::refusals::main::refuse_constructs::refuse_constructs;
use sqlquack_diagnostics::refusals::main::refuse_unparsed::refuse_unparsed;
use sqlquack_diagnostics::refusals::main::snowflake_rejections::snowflake_rejections;
use sqlquack_diagnostics::refusals::models::{Refusal, RefusalCategory};
use sqlquack_parse::parsing::main::parse_source::parse_source;
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
use sqlquack_semantics::lineage::models::{LineageContext, Nullability, SchemaColumn};
use sqlquack_semantics::lineage::types::LineageSchema;
use sqlquack_semantics::types::errors::SchemaError;
use sqlquack_semantics::types::main::parse_schema::parse_schema;
use sqlquack_semantics::types::main::source_tables::source_tables;
use sqlquack_semantics::types::models::Schema;
use sqlquack_syntax::tokens::main::node_location::node_location;
use sqlquack_syntax::tokens::models::SourceLines;
use sqlquack_syntax::{Dialect, Expression, ParseOptions};
use wasm_bindgen::prelude::wasm_bindgen;

/// Every stage's result for `sql` over the tables `sources` declares, as JSON. `settings` is
/// JSON `{"policy": TOML, "session": "NAME=VALUE" lines, "strict": bool, "utc_session": bool,
/// "compact": bool}`, `compact` printing each statement on one line as `convert --compact` does;
/// an empty policy is the built-in default, as `sqlquack convert --builtin-policy`.
#[wasm_bindgen]
pub fn analyze(sql: &str, sources: &str, settings: Option<String>) -> String {
    report(sql, sources, settings.as_deref().unwrap_or_default()).to_string()
}

/// A DuckDB error from running converted SQL, read back as the Snowflake error a building block
/// raised, as JSON: `{"errno", "sqlstate", "raise_id", "message"}`, or `{}` when it is DuckDB's own.
#[wasm_bindgen]
pub fn run_failure(message: &str) -> String {
    let Some(code) = duckdb_failure(message).and_then(|failure| failure.snowflake) else {
        return json!({}).to_string();
    };
    // The tagged text, `sqlquack:<errno>#<id>: <message>`, runs to the end of its line, or to the
    // closing quote DuckDB wraps it in (`... string '<tagged>' to INT32`). The message itself can
    // hold quotes, as in "Can't parse ...", so the closing quote is the line's last one.
    let tagged = message
        .split_once(ERROR_TAG_PREFIX)
        .map_or("", |(before, after)| {
            let line = after.lines().next().unwrap_or(after);
            match before.ends_with('\'') {
                true => line.rsplit_once('\'').map_or(line, |(text, _)| text),
                false => line,
            }
        });
    let text = tagged.split_once(TAG_END).map_or(tagged, |(_, text)| text);
    json!({
        "errno": code.errno,
        "sqlstate": code.sqlstate,
        "raise_id": raise_id(message),
        "message": text,
    })
    .to_string()
}

/// `CREATE TABLE` for each table `sources` declares, in the DuckDB types converted SQL reads them as,
/// as JSON: `{"tables": [...]}`, or `{"error": ...}`.
#[wasm_bindgen]
pub fn fixture_tables(sources: &str) -> String {
    let catalog = match catalog(sources) {
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
                        Err(_) if column.data_type == sqlquack_snowflake_types::typing::models::SnowflakeType::SemiStructured => {
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

/// The schema's relations as conversion types them, and as name checks resolve them.
#[derive(Debug, Default)]
struct Catalog {
    schema: Schema,
    columns: BTreeMap<String, Vec<SchemaColumn>>,
    /// Why a source is left out, as `sqlquack convert` warns about it.
    warnings: Vec<String>,
}

impl LineageSchema for Catalog {
    fn columns(&self, relation: &str) -> Option<&[SchemaColumn]> {
        self.columns.get(relation).map(Vec::as_slice)
    }
}

/// The source file `text`, read as `sqlquack convert` reads a project's `sources/*.yml`: a source
/// left out for a column without a type SQLQuack reads is a warning, and the rest still declare.
fn catalog(text: &str) -> Result<Catalog, String> {
    if text.trim().is_empty() {
        return Ok(Catalog::default());
    }
    let found = source_tables(text)?;
    let relations: serde_json::Map<String, Value> = found.tables.into_iter().collect();
    let schema =
        parse_schema(&json!({ "relations": relations }).to_string()).map_err(
            |error| match error {
                SchemaError::Shape(reason) | SchemaError::Declaration(reason) => reason,
            },
        )?;
    let columns = schema
        .relations
        .iter()
        .map(|(relation, columns)| {
            let named = columns
                .iter()
                .map(|column| SchemaColumn {
                    name: column.name.clone(),
                    quoted: false,
                    data_type: Some(column.data_type.to_string()),
                    nullability: Nullability::Unknown,
                })
                .collect();
            (relation.clone(), named)
        })
        .collect();
    Ok(Catalog {
        schema,
        columns,
        warnings: found.warnings,
    })
}

/// The playground's policy and session settings.
#[derive(Default, Deserialize)]
#[serde(default)]
struct Settings {
    policy: String,
    session: String,
    strict: bool,
    utc_session: bool,
    compact: bool,
}

/// Conversion options for `settings` as `sqlquack convert` chooses them, and the matched profile;
/// a session no recorded profile covers is a refusal, as on the command line.
fn chosen_options(
    settings: &Settings,
) -> Result<Result<(ConversionOptions, String), String>, String> {
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

/// Every stage's result for `sql` over `sources`, with a warning for each source left out.
fn report(sql: &str, sources: &str, settings: &str) -> Value {
    let catalog = match catalog(sources) {
        Ok(catalog) => catalog,
        Err(error) => return json!({ "sources_error": error }),
    };
    let mut report = stages(sql, &catalog, settings);
    if !catalog.warnings.is_empty() {
        report["warnings"] = json!(catalog.warnings);
    }
    report
}

/// Each stage's result for `sql` over `catalog`, as JSON.
fn stages(sql: &str, catalog: &Catalog, settings: &str) -> Value {
    let settings: Settings = match settings.trim() {
        "" => Settings::default(),
        text => match serde_json::from_str(text) {
            Ok(settings) => settings,
            Err(error) => return json!({ "settings_error": error.to_string() }),
        },
    };
    let session = match chosen_options(&settings) {
        Ok(session) => session,
        Err(error) => return json!({ "settings_error": error }),
    };
    let lines = SourceLines::new(sql);

    // 1. Parse as Snowflake.
    let statements = match parse_source(sql, Dialect::Snowflake) {
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
                schema: catalog,
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
                "sql": match settings.compact {
                    true => done.sql.clone(),
                    false => layout_sql(&done.sql),
                },
                "columns": done.columns.iter().map(|column| json!({
                    "name": column.name,
                    "type": column.data_type.to_string(),
                })).collect::<Vec<_>>(),
                // Each known divergence the policy allows and the conversion relies on, as
                // `sqlquack convert` names it on standard error, with its reason. An id such as
                // `declared_offset:REL.COL` is allowed under the part before the colon, as the
                // command line's `assumption_id` reads it.
                "divergences": done.provenance.divergences.iter().map(|(id, constructs)| {
                    let allowed = options
                        .divergences
                        .get(id.split_once(':').map_or(id.as_str(), |(base, _)| base));
                    json!({
                        "id": id,
                        "constructs": constructs,
                        "reason": allowed.map(|allowed| allowed.reason.as_str()),
                        "assumption": allowed.is_some_and(|allowed| allowed.assumption),
                    })
                }).collect::<Vec<_>>(),
                // Where each raise ID in the SQL comes from, so a run's error points at the source.
                "raises": done.raises,
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
