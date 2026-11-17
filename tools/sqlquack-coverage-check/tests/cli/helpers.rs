use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use sha2::{Digest, Sha256};

/// The form every fixture case is bound to.
const FORM: &str = "db.query.select";

fn sha256(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("write");
}

/// Runs `syntax-coverage` in this checkout, so the parser tree is read from its Git repository.
pub(crate) fn syntax_coverage(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_syntax-coverage"))
        .args(arguments)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("syntax-coverage runs")
}

/// A Databricks root with one approved query form, its one case `sql`, and `allowlist`.
pub(crate) fn approved_root(root: &Path, sql: &str, allowlist: &str) {
    let grammar: &str = "SELECT <expr>";
    let filler: String = "a".repeat(64);
    write(
        root,
        "dialect.toml",
        "format = \"sqlquack-syntax-dialect/1\"\nname = \"databricks\"\nform_prefix = \"db\"\n\
         families = [\"query\"]\nreceipt_format = \"sqlquack-syntax-receipt/2\"\n\
         parser_path = \"crates/sqlquack-syntax-databricks\"\nlocal_parser = \"databricks\"\n\
         syntax_codes = [\"42601\"]\n",
    );
    write(
        root,
        "sources.toml",
        &format!(
            "[[source]]\nid = \"doc.select\"\nurl = \"https://docs.example.com/select\"\n\
             title = \"SELECT\"\nfamily = \"query\"\nretrieved_utc = \"2026-11-03T20:00:00Z\"\n\
             page_sha256 = \"{filler}\"\nsyntax_sha256 = \"{filler}\"\nsyntax_blocks = 1\n\
             status = \"forms\"\ndecisions = []\n"
        ),
    );
    write(root, "indexes.toml", "");
    write(root, "limits.toml", "");
    write(root, "screen.toml", allowlist);
    write(
        root,
        "catalogue/query.toml",
        &format!(
            "[[form]]\nid = \"{FORM}\"\nfamily = \"query\"\nsource = \"doc.select\"\n\
             anchor = \"Syntax\"\nkind = \"statement\"\ngrammar = [\"{grammar}\"]\n\
             grammar_sha256 = \"{}\"\nuses = []\nstatus = \"catalogued\"\n",
            sha256(grammar)
        ),
    );
    write(
        root,
        "obligations/query.toml",
        &format!("[[obligation]]\nid = \"{FORM}.o001\"\nkind = \"positive\"\nrequired = true\n"),
    );
    let case: Value = serde_json::json!({
        "case_id": format!("{FORM}.k000000000001"),
        "obligations": [format!("{FORM}.o001")],
        "sql": sql,
        "sql_sha256": sha256(sql),
        "setup": [],
        "setup_sha256": sha256("[]"),
        "session": {},
        "session_sha256": sha256("{}"),
        "capture": "execute_autocommit_in_throwaway_database",
        "expect": "accept",
    });
    write(root, "cases/query.jsonl", &format!("{case}\n"));
    let root_text: &str = root.to_str().expect("root path");
    let family: Output = syntax_coverage(&["family", "query", "--json", "--root", root_text]);
    let forms: Value = serde_json::from_slice(&family.stdout).expect("family JSON");
    write(
        root,
        "reviews/query.toml",
        &format!(
            "[[review]]\nid = \"db-query-001\"\nfamily = \"query\"\n\
             parser_tree = \"0123456789abcdef0123456789abcdef01234567\"\n\
             reviewed_utc = \"2026-11-17T00:00:00Z\"\nverdict = \"approved\"\n\
             forms = [\"{FORM}\"]\nnote = \"fixture\"\n\n[review.fingerprints]\n\
             \"{FORM}\" = \"{}\"\n\n[review.definitions]\n\"{FORM}\" = \"{}\"\n",
            forms[0]["fingerprint"].as_str().expect("fingerprint"),
            forms[0]["definition_sha256"].as_str().expect("definition"),
        ),
    );
}
