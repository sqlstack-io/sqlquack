use std::process::Command;

use crate::test_types::{BinaryOutcome, SourceVariantOutcome};

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
