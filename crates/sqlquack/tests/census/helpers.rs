use std::path::Path;
use std::process::Command;

use crate::test_types::{
    BinaryOutcome, ConvertedFile, DeclaredOutcome, DefaultOutcome, SourceVariantOutcome,
    UpstreamOutcome,
};

const ORDERS_SQL: &str = "SELECT customer_id, COUNT(*) AS orders FROM __ref(\"orders\") \
                          GROUP BY customer_id";
const PRODUCTS_SQL: &str = "SELECT sku, price::NUMBER(10, 2) AS price FROM products";

/// Writes the source file `sources` into the project folder `project`.
fn write_sources(project: &Path, sources: &str) {
    std::fs::create_dir_all(project.join("sources")).expect("sources folder created");
    std::fs::write(project.join("sources/tables.yml"), sources).expect("sources written");
}

pub(crate) fn run_on_shop(arguments: &[&str]) -> BinaryOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("orders.sql"), ORDERS_SQL).expect("fixture written");
    std::fs::write(directory.path().join("products.sql"), PRODUCTS_SQL).expect("fixture written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(arguments)
        .arg(directory.path())
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    BinaryOutcome {
        exit_code: output.status.code(),
        files: report["totals"]["files"].as_u64(),
        unresolved_name: report["refusals"]["unresolved_name"].as_u64(),
        completion_reported: String::from_utf8_lossy(&output.stderr)
            .contains("sqlquack census: done, 2 files"),
        slowest_reported: String::from_utf8_lossy(&output.stderr)
            .contains("sqlquack census: slowest files:\n  "),
        names_printed: [&output.stdout, &output.stderr].iter().any(|stream| {
            let text = String::from_utf8_lossy(stream).to_ascii_lowercase();
            text.contains("orders") || text.contains("products")
        }),
    }
}

const FUZZY_NAMES_SQL: &str = "SELECT customer_id, SOUNDEX(name) AS sound FROM customers";
const NAME_REPORT_SQL: &str =
    "SELECT customer_id, UPPER(sound) AS sound FROM __ref(\"fuzzy_names\")";
const CUSTOMERS_SOURCE: &str = "  - name: customers\n    columns:\n      - name: customer_id\n        \
                                type: NUMBER(38,0)\n      - name: name\n        type: VARCHAR\n";
const FUZZY_NAMES_SOURCE: &str = "  - name: __sqlquack_relation__fuzzy_names\n    columns:\n      \
                                  - name: customer_id\n        type: NUMBER(38,0)\n      \
                                  - name: sound\n        type: VARCHAR\n";

/// The chained census of a refused model and its reader, the project typing the model if `typed`.
pub(crate) fn run_on_refused_upstream(typed: bool, flags: &[&str]) -> UpstreamOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("models");
    std::fs::create_dir(&models).expect("models directory created");
    std::fs::write(models.join("fuzzy_names.sql"), FUZZY_NAMES_SQL).expect("fixture written");
    std::fs::write(models.join("name_report.sql"), NAME_REPORT_SQL).expect("fixture written");
    let sources = [CUSTOMERS_SOURCE, FUZZY_NAMES_SOURCE][..1 + usize::from(typed)].concat();
    write_sources(directory.path(), &format!("sources:\n{sources}"));
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args([
            "census",
            "--detailed",
            "--builtin-policy",
            "--chain-models",
            "--project",
        ])
        .arg(directory.path())
        .args(flags)
        .arg(&models)
        .output()
        .expect("sqlquack runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    UpstreamOutcome {
        exit_code: output.status.code(),
        unit_testable: report["refusals"]["convertible"].as_u64(),
        strict: report["refusals"]["convertible_strict"].as_u64(),
        labels: report["blockers"]["upstream_refused"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|file| file["labels"].as_array().cloned().unwrap_or_default())
            .filter_map(|label| label.as_str().map(str::to_owned))
            .collect(),
        text_counts: stdout
            .lines()
            .find(|line| line.trim_start().starts_with("convertible: strict"))
            .map(|line| line.trim().to_owned()),
        plan_bounds: report["plan"]["unit_testable_upper_bound"]
            .as_u64()
            .zip(report["plan"]["strict_upper_bound"].as_u64())
            .map(|(unit, strict)| [unit, strict]),
    }
}

const EVENTS_SQL: &str = "SELECT payload:kind = PARSE_JSON('\"order\"') AS is_order FROM events";

/// Runs the census, with `flags`, over a model comparing a source VARIANT, `payload` its column's YAML.
pub(crate) fn run_on_events(payload: &str, flags: &[&str]) -> SourceVariantOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("models");
    std::fs::create_dir(&models).expect("models directory created");
    std::fs::write(models.join("order_events.sql"), EVENTS_SQL).expect("fixture written");
    write_sources(
        directory.path(),
        &format!("sources:\n  - name: events\n    columns:\n{payload}"),
    );
    let policy = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sqlquack.toml");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sqlquack"));
    command
        .args([
            "census",
            "--detailed",
            "--json",
            "--chain-models",
            "--project",
        ])
        .arg(directory.path())
        .arg("--policy")
        .arg(&policy);
    let output = command
        .args(flags)
        .arg(&models)
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    SourceVariantOutcome {
        exit_code: output.status.code(),
        convertible: report["refusals"]["convertible"].as_u64(),
        labelled: report["provenance"]["divergences"]["source_variant_json at node:eq"].as_u64(),
    }
}

const PLACED_SQL: &str = "SELECT CAST(placed_on AS TIMESTAMP_TZ) AS placed_at FROM orders";

/// Runs the census, with `flags`, over a model reading a naive column as TIMESTAMP_TZ.
pub(crate) fn run_on_placed_orders(flags: &[&str]) -> SourceVariantOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("models");
    std::fs::create_dir(&models).expect("models directory created");
    std::fs::write(models.join("placed_orders.sql"), PLACED_SQL).expect("fixture written");
    write_sources(
        directory.path(),
        "sources:\n  - name: orders\n    columns:\n      - name: placed_on\n        type: DATE\n",
    );
    let policy = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sqlquack.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["census", "--detailed", "--json", "--project"])
        .arg(directory.path())
        .arg("--policy")
        .arg(&policy)
        .args(flags)
        .arg(&models)
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    SourceVariantOutcome {
        exit_code: output.status.code(),
        convertible: report["refusals"]["convertible"].as_u64(),
        labelled: report["provenance"]["divergences"]["session_timezone_utc at node:cast"].as_u64(),
    }
}

/// The warnings the census prints for `flags` over a refused model and its reader.
pub(crate) fn census_warnings(flags: &[&str]) -> Vec<String> {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("models");
    std::fs::create_dir(&models).expect("models directory created");
    std::fs::write(models.join("fuzzy_names.sql"), FUZZY_NAMES_SQL).expect("fixture written");
    std::fs::write(models.join("name_report.sql"), NAME_REPORT_SQL).expect("fixture written");
    write_sources(directory.path(), &format!("sources:\n{CUSTOMERS_SOURCE}"));
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args([
            "census",
            "--detailed",
            "--builtin-policy",
            "--json",
            "--chain-models",
            "--project",
        ])
        .arg(directory.path())
        .args(flags)
        .arg(&models)
        .output()
        .expect("sqlquack runs");
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter(|line| line.contains("warning:"))
        .map(str::to_owned)
        .collect()
}

const CURSOR_SQL: &str = "SELECT order_id FROM orders WHERE order_id >= __cursor_start() \
                          AND order_id < __cursor_end()";

/// The census's unit-testable count over a model filtering by cursor intrinsics, with `flags`.
pub(crate) fn cursor_model_convertible(flags: &[&str]) -> Option<u64> {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("models");
    std::fs::create_dir(&models).expect("models directory created");
    std::fs::write(models.join("new_orders.sql"), CURSOR_SQL).expect("fixture written");
    write_sources(
        directory.path(),
        "sources:\n  - name: orders\n    columns:\n      - name: order_id\n        type: NUMBER(38,0)\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args([
            "census",
            "--detailed",
            "--builtin-policy",
            "--json",
            "--project",
        ])
        .arg(directory.path())
        .args(flags)
        .arg(&models)
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    report["refusals"]["convertible"].as_u64()
}

const PLACED_DATE_SQL: &str = "SELECT TO_DATE(placed_at) AS placed_on FROM orders";
const DECLARED_ORDERS_SOURCE: &str = "  - name: orders\n    columns:\n      - name: placed_at\n        \
                                      type: TIMESTAMP_TZ(9)\n        offset: \"+00:00\"\n";

/// The census of a model reading a declared-UTC column, optionally also declaring its output.
pub(crate) fn run_on_declared_orders(declare_model: bool, flags: &[&str]) -> DeclaredOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("models");
    std::fs::create_dir(&models).expect("models directory created");
    std::fs::write(models.join("placed_dates.sql"), PLACED_DATE_SQL).expect("fixture written");
    let model = "  - name: __sqlquack_relation__placed_dates\n    columns:\n      \
                 - name: placed_on\n        type: TIMESTAMP_TZ(9)\n        offset: UTC\n";
    write_sources(
        directory.path(),
        &format!(
            "sources:\n{DECLARED_ORDERS_SOURCE}{}",
            ["", model][usize::from(declare_model)]
        ),
    );
    let policy = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sqlquack.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["census", "--detailed", "--json", "--project"])
        .arg(directory.path())
        .arg("--policy")
        .arg(&policy)
        .args(flags)
        .arg(&models)
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    let keys = |value: &serde_json::Value| -> Vec<String> {
        value
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    };
    DeclaredOutcome {
        exit_code: output.status.code(),
        assumptions: keys(&report["refusals"]["assumptions"]),
        divergences: keys(&report["provenance"]["divergences"]),
        model_declaration_refused: String::from_utf8_lossy(&output.stderr)
            .contains("offsets may be declared only for source columns"),
    }
}

const CUSTOMER_IDS_SQL: &str = "SELECT customer_id FROM customers";

/// The files census writes under `SQLQUACK_CONVERTED_DIR` for a converted and a refused model.
pub(crate) fn converted_files(set_variable: bool) -> Vec<ConvertedFile> {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("models");
    let converted = directory.path().join("converted");
    std::fs::create_dir(&models).expect("models directory created");
    std::fs::create_dir(&converted).expect("converted directory created");
    std::fs::write(models.join("customer_ids.sql"), CUSTOMER_IDS_SQL).expect("fixture written");
    std::fs::write(models.join("fuzzy_names.sql"), FUZZY_NAMES_SQL).expect("fixture written");
    write_sources(directory.path(), &format!("sources:\n{CUSTOMERS_SOURCE}"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_sqlquack"));
    command
        .args([
            "census",
            "--detailed",
            "--builtin-policy",
            "--json",
            "--project",
        ])
        .arg(directory.path())
        .arg(&models)
        .env_remove("SQLQUACK_CONVERTED_DIR")
        .envs(set_variable.then_some(("SQLQUACK_CONVERTED_DIR", &converted)))
        .output()
        .expect("sqlquack runs");
    let mut files: Vec<ConvertedFile> = std::fs::read_dir(&converted)
        .expect("converted directory read")
        .map(|entry| {
            let path = entry.expect("directory entry").path();
            let name = path
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            ConvertedFile {
                model: name.split("_models_").last().unwrap_or_default().to_owned(),
                holds_select: std::fs::read_to_string(&path)
                    .expect("converted SQL read")
                    .to_ascii_uppercase()
                    .contains("SELECT"),
            }
        })
        .collect();
    files.sort_by(|left, right| left.model.cmp(&right.model));
    files
}

const COMPILED_STAGING_SQL: &str = "with source as (\n\n    select * from analytics.raw.orders\n\n),\n\n\
                                    renamed as (\n\n    select id as order_id, status from source\n\n)\n\n\
                                    select * from renamed\n";
const COMPILED_MART_SQL: &str = "select status, count(order_id) as orders\n\
                                 from \"ANALYTICS\".\"DBT_PROD\".\"STG_ORDERS\"\ngroup by status\n";
const RAW_ORDERS_SOURCES: &str = "sources:\n  - name: orders\n    database: analytics\n    schema: raw\n    \
                                  columns:\n      - name: id\n        type: NUMBER(38,0)\n      \
                                  - name: status\n        type: VARCHAR\n";

/// The unit-testable and strict counts of a census of dbt-compiled models, with `flags`.
pub(crate) fn compiled_dbt_counts(flags: &[&str]) -> (Option<u64>, Option<u64>) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("target/compiled/shop/models");
    std::fs::create_dir_all(models.join("staging")).expect("staging directory created");
    std::fs::create_dir_all(models.join("marts")).expect("marts directory created");
    std::fs::write(models.join("staging/stg_orders.sql"), COMPILED_STAGING_SQL)
        .expect("fixture written");
    std::fs::write(models.join("marts/order_counts.sql"), COMPILED_MART_SQL)
        .expect("fixture written");
    write_sources(directory.path(), RAW_ORDERS_SOURCES);
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args([
            "census",
            "--detailed",
            "--builtin-policy",
            "--json",
            "--project",
        ])
        .arg(directory.path())
        .args(flags)
        .arg(directory.path().join("target/compiled"))
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    (
        report["refusals"]["convertible"].as_u64(),
        report["refusals"]["convertible_strict"].as_u64(),
    )
}

const CUSTOMER_NAMES_SQL: &str = "SELECT customer_id, name FROM customers";

/// The default census, text and `--json`, of `models` (its first `files`), paths relative.
pub(crate) fn default_census(typed: bool, files: usize) -> DefaultOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(directory.path().join("models")).expect("models directory created");
    let models = [
        ("customer_names.sql", CUSTOMER_NAMES_SQL),
        ("fuzzy_names.sql", FUZZY_NAMES_SQL),
        ("name_report.sql", NAME_REPORT_SQL),
    ];
    for (name, sql) in &models[..files] {
        std::fs::write(directory.path().join("models").join(name), sql).expect("fixture written");
    }
    let sources = [CUSTOMERS_SOURCE, FUZZY_NAMES_SOURCE][..1 + usize::from(typed)].concat();
    write_sources(directory.path(), &format!("sources:\n{sources}"));
    let run = |format: &str| {
        Command::new(env!("CARGO_BIN_EXE_sqlquack"))
            .current_dir(directory.path())
            .args([
                "census",
                "--builtin-policy",
                "--project",
                ".",
                "--format",
                format,
            ])
            .arg("models")
            .output()
            .expect("sqlquack runs")
    };
    let text = run("text");
    let report: serde_json::Value = serde_json::from_slice(&run("json").stdout).unwrap_or_default();
    DefaultOutcome {
        exit_code: text.status.code(),
        text_lines: String::from_utf8_lossy(&text.stdout)
            .lines()
            .filter(|line| {
                [
                    "error[",
                    "warning:",
                    " --> ",
                    "Converted",
                    "Blocked",
                    "Converts",
                ]
                .iter()
                .any(|start| line.starts_with(start))
                    || line.contains("= note:")
            })
            .map(str::to_owned)
            .collect(),
        json_ok: report["ok"].as_bool(),
        json_codes: report["blocked"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|file| file["diagnostics"].as_array().cloned().unwrap_or_default())
            .filter_map(|diagnostic| diagnostic["code"].as_str().map(str::to_owned))
            .collect(),
        json_upstream: report["upstream"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|warning| warning["path"].as_str().map(str::to_owned))
            .collect(),
    }
}
