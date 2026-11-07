use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::check::models::{LocalFailure, LocalVerdict};
use crate::formats::constants::FAMILIES;

/// The parser tree every family review in the fixture binds.
pub(crate) const TREE: &str = "0123456789abcdef0123456789abcdef01234567";
/// Another committed parser tree.
pub(crate) const OTHER_TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";
const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub(crate) fn sha256(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("write");
}

pub(crate) fn read(root: &Path, path: &str) -> String {
    std::fs::read_to_string(root.join(path)).expect("read")
}

pub(crate) fn replace(root: &Path, path: &str, from: &str, to: &str) {
    let text = read(root, path);
    assert!(text.contains(from), "{path} holds {from}");
    write(root, path, &text.replacen(from, to, 1));
}

/// Drops every line of `path` that contains `fragment`.
pub(crate) fn drop_lines(root: &Path, path: &str, fragment: &str) {
    let text = read(root, path);
    assert!(text.contains(fragment), "{path} holds {fragment}");
    let kept: String = text
        .lines()
        .filter(|line| !line.contains(fragment))
        .map(|line| format!("{line}\n"))
        .collect();
    write(root, path, &kept);
}

/// SQLQuack as the fixture needs it: `SELECT FROM` is rejected, everything else is structural.
pub(crate) fn structural(sql: &str) -> LocalVerdict {
    vec![accepted_as(sql), LocalVerdict::Rejected].swap_remove(usize::from(sql == "SELECT FROM"))
}

/// A structural acceptance whose tree serialises as `tree`.
fn accepted_as(tree: &str) -> LocalVerdict {
    LocalVerdict::Accepted {
        opaque: false,
        tree_sha256: Some(sha256(tree)),
    }
}

/// `structural`, except that `SELECT 17` parses into another tree.
pub(crate) fn reshaped(sql: &str) -> LocalVerdict {
    vec![structural(sql), accepted_as("another tree")].swap_remove(usize::from(sql == "SELECT 17"))
}

/// `structural`, except that `SELECT 17` is rejected.
pub(crate) fn rejecting_select_17(sql: &str) -> LocalVerdict {
    vec![structural(sql), LocalVerdict::Rejected].swap_remove(usize::from(sql == "SELECT 17"))
}

/// `structural`, except that `SELECT FROM` crashes instead of being rejected.
pub(crate) fn crashing_on_select_from(sql: &str) -> LocalVerdict {
    vec![structural(sql), crashing(sql)].swap_remove(usize::from(sql == "SELECT FROM"))
}

/// Rewrites the query review to list both query forms with their fingerprints under `structural`.
pub(crate) fn pin_fingerprints(root: &Path) {
    let coverage = crate::formats::main::load_coverage::load_coverage(root).expect("load");
    let check =
        crate::check::main::check_coverage::check_coverage(&coverage, Some(TREE), &structural);
    let pins: Vec<String> = check
        .forms
        .iter()
        .filter(|form| form.family == "query")
        .map(|form| format!("\"{}\" = \"{}\"", form.id, form.fingerprint))
        .collect();
    replace(
        root,
        "reviews/query.toml",
        "verdict = \"approved\"\n",
        &format!(
            "verdict = \"approved\"\nforms = [\"sf.query.select\", \"sf.query.select.top\"]\n\
             fingerprints = {{ {} }}\n",
            pins.join(", ")
        ),
    );
}

/// Rewrites the query review to bind each query form by fingerprint and definition.
pub(crate) fn pin_definitions(root: &Path) {
    let coverage = crate::formats::main::load_coverage::load_coverage(root).expect("load");
    let check =
        crate::check::main::check_coverage::check_coverage(&coverage, Some(TREE), &structural);
    let query: Vec<&crate::check::models::FormResult> = check
        .forms
        .iter()
        .filter(|form| form.family == "query")
        .collect();
    let pins = |value: fn(&crate::check::models::FormResult) -> &str| -> String {
        query
            .iter()
            .map(|form| format!("\"{}\" = \"{}\"", form.id, value(form)))
            .collect::<Vec<String>>()
            .join(", ")
    };
    let review: String = format!(
        "[[review]]\nid = \"query-1\"\nreviewer_lane = \"auditor\"\nfamily = \"query\"\n\
         parser_tree = \"{TREE}\"\nreviewed_utc = \"2026-11-04T09:00:00Z\"\n\
         verdict = \"approved\"\nforms = [\"sf.query.select\", \"sf.query.select.top\"]\n\
         fingerprints = {{ {} }}\ndefinitions = {{ {} }}\n",
        pins(|form| &form.fingerprint),
        pins(|form| &form.definition_sha256),
    );
    write(root, "reviews/query.toml", &review);
}

/// A local parse that crashes on everything.
pub(crate) fn crashing(_sql: &str) -> LocalVerdict {
    LocalVerdict::Failed {
        failure: LocalFailure::Crash,
    }
}

/// A local parse that runs past its time bound on everything.
pub(crate) fn hanging(_sql: &str) -> LocalVerdict {
    LocalVerdict::Failed {
        failure: LocalFailure::Timeout,
    }
}

/// A parser that rejects everything.
pub(crate) fn rejecting(_sql: &str) -> LocalVerdict {
    LocalVerdict::Rejected
}

/// Appends `line` to the receipts file.
pub(crate) fn append_receipt(root: &Path, line: &str) {
    let receipts: String = read(root, "receipts/run-1.jsonl");
    write(root, "receipts/run-1.jsonl", &format!("{receipts}{line}\n"));
}

/// A parser that accepts everything, through a Command or Raw fallback.
pub(crate) fn opaque(sql: &str) -> LocalVerdict {
    LocalVerdict::Accepted {
        opaque: true,
        tree_sha256: Some(sha256(sql)),
    }
}

/// A case line, its hashes computed from its SQL, setup and session.
pub(crate) fn case_line(case_id: &str, obligation: &str, sql: &str, setup: &[&str]) -> String {
    case_in_zone(case_id, obligation, sql, setup, "UTC")
}

/// A case line whose session sets `timezone`.
pub(crate) fn case_in_zone(
    case_id: &str,
    obligation: &str,
    sql: &str,
    setup: &[&str],
    timezone: &str,
) -> String {
    let session: BTreeMap<String, Value> =
        BTreeMap::from([("TIMEZONE".to_owned(), Value::from(timezone))]);
    let setup_json: String = serde_json::to_string(setup).expect("setup");
    let session_json: String = serde_json::to_string(&session).expect("session");
    serde_json::json!({
        "case_id": case_id,
        "obligations": [obligation],
        "sql": sql,
        "sql_sha256": sha256(sql),
        "setup": setup,
        "setup_sha256": sha256(&setup_json),
        "session": session,
        "session_sha256": sha256(&session_json),
        "capture": "execute",
    })
    .to_string()
}

/// A receipt line for `case`, a case line, recording `phase`.
pub(crate) fn receipt_line(case: &str, phase: &str) -> String {
    let case: Value = serde_json::from_str(case).expect("case");
    serde_json::json!({
        "format": "sqlquack-syntax-receipt/1",
        "case_id": case["case_id"],
        "sql_sha256": case["sql_sha256"],
        "setup_sha256": case["setup_sha256"],
        "session_sha256": case["session_sha256"],
        "session_parameters": case["session"],
        "capture_method": "execute",
        "run_id": "run-1",
        "query_id": format!("query-{}", case["case_id"].as_str().unwrap_or_default()),
        "warehouse_version": "10.40.100",
        "recorded_at": "2026-11-03T22:45:26Z",
        "outcome": phase,
        "phase": phase,
        "phase_basis": "errno",
        "raw_record_sha256": SHA_A,
        "run_audit_sha256": SHA_A,
        "packet_sha256": SHA_A,
    })
    .to_string()
}

pub(crate) fn form(id: &str, grammar: &str) -> String {
    format!(
        "[[form]]\nid = \"{id}\"\nfamily = \"query\"\nsource = \"doc.select\"\n\
         anchor = \"Syntax\"\nkind = \"statement\"\ngrammar = [\"{grammar}\"]\n\
         grammar_sha256 = \"{}\"\nuses = []\nstatus = \"catalogued\"\n",
        sha256(grammar)
    )
}

pub(crate) fn obligation(id: &str, kind: &str) -> String {
    format!("[[obligation]]\nid = \"{id}\"\nkind = \"{kind}\"\nrequired = true\n")
}

/// Approved reviews bound to the files as written now: the query family and the pages.
pub(crate) fn review(root: &Path) {
    write(
        root,
        "reviews/query.toml",
        &format!(
            "[[review]]\nid = \"query-1\"\nreviewer_lane = \"c\"\nfamily = \"query\"\n\
             catalogue_sha256 = \"{}\"\nobligations_sha256 = \"{}\"\nparser_tree = \"{TREE}\"\n\
             reviewed_utc = \"2026-11-04T09:00:00Z\"\nverdict = \"approved\"\n",
            sha256(&read(root, "catalogue/query.toml")),
            sha256(&read(root, "obligations/query.toml")),
        ),
    );
    write(
        root,
        "reviews/pages.toml",
        &format!(
            "[[review]]\nid = \"pages-1\"\nreviewer_lane = \"backend\"\nfamily = \"\"\n\
             sources_sha256 = \"{}\"\nreviewed_utc = \"2026-11-04T09:00:00Z\"\n\
             verdict = \"approved\"\n[[review.exclusion]]\nsource = \"doc.billing\"\n\
             reason = \"documents billing views, not syntax\"\n",
            sha256(&read(root, "sources.toml")),
        ),
    );
}

/// A tree in which both query forms verify against `structural` and `TREE`.
pub(crate) fn verified_tree(root: &Path) {
    for family in FAMILIES {
        write(root, &format!("catalogue/{family}.toml"), "");
        write(root, &format!("obligations/{family}.toml"), "");
        write(root, &format!("cases/{family}.jsonl"), "");
    }
    write(
        root,
        "sources.toml",
        &format!(
            "[[source]]\nid = \"doc.select\"\nurl = \"https://docs.example.com/select\"\n\
             title = \"SELECT\"\nfamily = \"query\"\nretrieved_utc = \"2026-11-03T20:00:00Z\"\n\
             page_sha256 = \"{SHA_A}\"\nsyntax_sha256 = \"{SHA_A}\"\nsyntax_blocks = 2\n\
             status = \"forms\"\ndecisions = []\n\
             [[source]]\nid = \"doc.billing\"\nurl = \"https://docs.example.com/billing\"\n\
             title = \"Billing\"\nfamily = \"\"\nretrieved_utc = \"2026-11-03T20:00:00Z\"\n\
             page_sha256 = \"{SHA_A}\"\nsyntax_sha256 = \"{SHA_A}\"\nsyntax_blocks = 0\n\
             status = \"out_of_scope\"\ndecisions = []\n"
        ),
    );
    write(root, "indexes.toml", "");
    write(
        root,
        "catalogue/query.toml",
        &[
            form("sf.query.select", "SELECT <expr>"),
            form("sf.query.select.top", "SELECT TOP <n> <expr>"),
        ]
        .concat(),
    );
    write(
        root,
        "obligations/query.toml",
        &[
            obligation("sf.query.select.o001", "positive"),
            obligation("sf.query.select.o002", "negative_neighbour"),
            obligation("sf.query.select.top.o001", "positive"),
        ]
        .concat(),
    );
    let cases: [String; 3] = [
        select_17(),
        case_line("select-from", "sf.query.select.o002", "SELECT FROM", &[]),
        case_line(
            "select-top",
            "sf.query.select.top.o001",
            "SELECT TOP 1 17",
            &[],
        ),
    ];
    write(
        root,
        "cases/query.jsonl",
        &format!("{}\n", cases.join("\n")),
    );
    let receipts: [String; 3] = [
        receipt_line(&cases[0], "accepted"),
        receipt_line(&cases[1], "syntax_error"),
        receipt_line(&cases[2], "accepted"),
    ];
    write(
        root,
        "receipts/run-1.jsonl",
        &format!("{}\n", receipts.join("\n")),
    );
    write(
        root,
        "limits.toml",
        &format!(
            "[[reading]]\nform = \"sf.query.select\"\ngrammar_sha256 = \"{SHA_A}\"\n\
             grammar = [\"SELECT <expr> read as one expression\"]\nreason = \"r\"\n\
             [[reading]]\nform = \"sf.query.select.top\"\ngrammar_sha256 = \"{SHA_A}\"\n\
             grammar = [\"SELECT TOP <n> <expr> read as one count\"]\nreason = \"r\"\n"
        ),
    );
    review(root);
}

/// The case of `sf.query.select.o001` as the verified tree writes it.
pub(crate) fn select_17() -> String {
    case_line("select-17", "sf.query.select.o001", "SELECT 17", &[SETUP])
}

/// The setup of `select-17`.
pub(crate) const SETUP: &str = "CREATE TABLE t (a INT)";

/// Replaces the case line of `select-17` with `case`, leaving its receipt as recorded.
pub(crate) fn rewrite_select_17(root: &Path, case: &str) {
    replace(root, "cases/query.jsonl", &select_17(), case);
}

/// A second query review with `verdict` and `tree` that excludes `sf.query.select.top`.
pub(crate) fn exclusion_review(root: &Path, verdict: &str, tree: &str) {
    let first: String = std::fs::read_to_string(root.join("reviews/query.toml")).expect("read");
    let second: String = first
        .replace("id = \"query-1\"", "id = \"query-2\"")
        .replace(
            "verdict = \"approved\"",
            &format!("verdict = \"{verdict}\""),
        )
        .replace(TREE, tree);
    write(
        root,
        "reviews/excluding.toml",
        &format!(
            "{second}[[review.exclusion]]\nform = \"sf.query.select.top\"\n\
             reason = \"valid only inside project files\"\n"
        ),
    );
}

/// The setup hash of `select-17`.
pub(crate) fn select_17_setup_sha256() -> String {
    sha256(&serde_json::to_string(&[SETUP]).expect("setup"))
}

/// `line`, a case or receipt line, with `fields` added or replaced.
pub(crate) fn with_fields(line: &str, fields: Value) -> String {
    let mut record: Value = serde_json::from_str(line).expect("line");
    for (key, value) in fields.as_object().expect("fields") {
        record[key] = value.clone();
    }
    record.to_string()
}

/// Lists `sf.query.select.o001` as unobservable, citing case `case_id` with the SHA-256 of `sql`.
pub(crate) fn unobservable(root: &Path, obligation: &str, case_id: &str, sql: &str) {
    write(
        root,
        "unobservable/query.toml",
        &format!(
            "[[unobservable]]\nobligation = \"{obligation}\"\nform = \"sf.query.select\"\n\
             construct = \"a construct\"\nreason = \"internal error 603 on every capture\"\n\
             cases = [{{ case_id = \"{case_id}\", sql_sha256 = \"{}\" }}]\n\
             runs = [\"run-1\"]\nprobes = []\n",
            sha256(sql)
        ),
    );
}

/// Lists `obligation` as never submitted, citing `case_id` with the hash of `sql`, with `runs`.
pub(crate) fn not_submitted(root: &Path, obligation: &str, case_id: &str, sql: &str, runs: &str) {
    write(
        root,
        "unobservable/query.toml",
        &format!(
            "[[unobservable]]\nobligation = \"{obligation}\"\nform = \"sf.query.select\"\n\
             construct = \"a construct\"\nreason = \"not submitted: held by the capture guard\"\n\
             cause = \"not_submitted\"\n\
             cases = [{{ case_id = \"{case_id}\", sql_sha256 = \"{}\" }}]\n\
             runs = [{runs}]\nprobes = []\n",
            sha256(sql)
        ),
    );
}

/// Adds a note to the obligation `id` of kind `kind`, an edit the obligations file alone sees.
pub(crate) fn note_obligation(root: &Path, id: &str, kind: &str) {
    let declared: String = format!("id = \"{id}\"\nkind = \"{kind}\"\nrequired = true\n");
    replace(
        root,
        "obligations/query.toml",
        &declared,
        &format!("{declared}note = \"edited after review\"\n"),
    );
}

/// Moves the catalogue form `id` to another anchor, an edit the catalogue file alone sees.
pub(crate) fn move_form(root: &Path, id: &str) {
    let declared: String = format!("id = \"{id}\"\nfamily = \"query\"\nsource = \"doc.select\"\n");
    replace(
        root,
        "catalogue/query.toml",
        &format!("{declared}anchor = \"Syntax\""),
        &format!("{declared}anchor = \"Syntax, edited\""),
    );
}

/// Adds a receipt recording `fields` for `retired-1`, a case id no form cites, of `sql`.
pub(crate) fn retired_receipt(root: &Path, sql: &str, fields: Value) {
    let case: String = case_line("retired-1", "sf.query.select.o001", sql, &[]);
    append_receipt(root, &with_fields(&receipt_line(&case, "accepted"), fields));
}

/// Writes the run report beside the receipts, listing `sql` among its inputs.
pub(crate) fn report_input(root: &Path, sql: &str) {
    let report: Value = serde_json::json!({
        "inputs": [{"sql": sql, "sql_sha256": sha256(sql)}],
    });
    write(root, "receipts/run-1.report.json", &report.to_string());
}
