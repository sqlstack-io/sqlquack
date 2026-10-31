use std::process::Command;

use crate::test_types::{BinaryOutcome, SourceVariantOutcome, UpstreamOutcome};

const ORDERS_SQL: &str = "SELECT customer_id, COUNT(*) AS orders FROM __ref(\"orders\") \
                          GROUP BY customer_id";
const PRODUCTS_SQL: &str = "SELECT sku, price::NUMBER(10, 2) AS price FROM products";

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
const CUSTOMERS_SCHEMA: &str =
    "\"CUSTOMERS\": [[\"CUSTOMER_ID\", \"NUMBER(38,0)\"], [\"NAME\", \"VARCHAR\"]]";
const FUZZY_NAMES_SCHEMA: &str = "\"__SQLQUACK_RELATION__FUZZY_NAMES\": \
                                  [[\"CUSTOMER_ID\", \"NUMBER(38,0)\"], [\"SOUND\", \"VARCHAR\"]]";

/// The chained census of a refused model and its reader, the schema typing the model if `typed`.
pub(crate) fn run_on_refused_upstream(typed: bool, flags: &[&str]) -> UpstreamOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("models");
    std::fs::create_dir(&models).expect("models directory created");
    std::fs::write(models.join("fuzzy_names.sql"), FUZZY_NAMES_SQL).expect("fixture written");
    std::fs::write(models.join("name_report.sql"), NAME_REPORT_SQL).expect("fixture written");
    let relations = [CUSTOMERS_SCHEMA, FUZZY_NAMES_SCHEMA][..1 + usize::from(typed)].join(", ");
    let schema = directory.path().join("schema.json");
    std::fs::write(&schema, format!("{{\"relations\": {{{relations}}}}}")).expect("schema written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["census", "--builtin-policy", "--chain-models", "--schema"])
        .arg(&schema)
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

/// Runs the census, with `flags`, over a model comparing a source VARIANT whose column is `payload`.
pub(crate) fn run_on_events(payload: &str, flags: &[&str]) -> SourceVariantOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let models = directory.path().join("models");
    std::fs::create_dir(&models).expect("models directory created");
    std::fs::write(models.join("order_events.sql"), EVENTS_SQL).expect("fixture written");
    let schema = directory.path().join("schema.json");
    std::fs::write(
        &schema,
        format!("{{\"relations\": {{\"EVENTS\": [{payload}]}}}}"),
    )
    .expect("schema written");
    let policy = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sqlquack.toml");
    let mut command = Command::new(env!("CARGO_BIN_EXE_sqlquack"));
    command
        .args(["census", "--json", "--chain-models", "--schema"])
        .arg(&schema)
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
    let schema = directory.path().join("schema.json");
    std::fs::write(
        &schema,
        "{\"relations\": {\"ORDERS\": [[\"PLACED_ON\", \"DATE\"]]}}",
    )
    .expect("schema written");
    let policy = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sqlquack.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["census", "--json", "--schema"])
        .arg(&schema)
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
    let schema = directory.path().join("schema.json");
    std::fs::write(
        &schema,
        format!("{{\"relations\": {{{CUSTOMERS_SCHEMA}}}}}"),
    )
    .expect("schema written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args([
            "census",
            "--builtin-policy",
            "--json",
            "--chain-models",
            "--schema",
        ])
        .arg(&schema)
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
    let schema = directory.path().join("schema.json");
    std::fs::write(
        &schema,
        "{\"relations\": {\"ORDERS\": [[\"ORDER_ID\", \"NUMBER(38,0)\"]]}}",
    )
    .expect("schema written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["census", "--builtin-policy", "--json", "--schema"])
        .arg(&schema)
        .args(flags)
        .arg(&models)
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
    report["refusals"]["convertible"].as_u64()
}
