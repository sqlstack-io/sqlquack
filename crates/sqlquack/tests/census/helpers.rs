use std::process::Command;

use crate::test_types::BinaryOutcome;

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
    }
}
