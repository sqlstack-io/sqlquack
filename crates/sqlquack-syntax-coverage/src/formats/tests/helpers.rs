use std::path::Path;

use sha2::{Digest, Sha256};

use crate::formats::constants::FAMILIES;
use crate::formats::models::Receipt;

const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

pub(crate) fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("write");
}

pub(crate) fn replace(root: &Path, path: &str, from: &str, to: &str) {
    let text = std::fs::read_to_string(root.join(path)).expect("read");
    assert!(text.contains(from), "{path} holds {from}");
    write(root, path, &text.replacen(from, to, 1));
}

/// A query case whose hashes were computed independently, in Python, with compact JSON.
pub(crate) fn case_line() -> String {
    concat!(
        r#"{"case_id":"query.select-17","obligations":["sf.query.select.o001"],"#,
        r#""sql":"SELECT 17","#,
        r#""sql_sha256":"ef83e1f6253a9a3e957db2ad55395a2ff7743a020a4f73c6085dbcbc5cb09ac7","#,
        r#""setup":["CREATE TABLE orders (\"é\" INT)","INSERT INTO orders VALUES (17)"],"#,
        r#""setup_sha256":"c5305748108cd15f217a3b64bd0dc9a891afc95159da14577e7ceb216bd1b9cc","#,
        r#""session":{"TIMEZONE":"UTC","USE_CACHED_RESULT":false,"STATEMENT_TIMEOUT_IN_SECONDS":30},"#,
        r#""session_sha256":"126d70f8bbd834c0e341349f5804accba9c481e8c6acf3dfe1ac15b51414f21f","#,
        r#""capture":"execute"}"#,
    )
    .to_owned()
}

/// The SHA-256 of a receipt line, as a superseding receipt or a withdrawal names it.
pub(crate) fn line_sha256(line: &str) -> String {
    Sha256::digest(line.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The receipt above, now recording a semantic error, superseding the line `old` hashes to.
pub(crate) fn superseding_line(old: &str) -> String {
    receipt_line().replace(
        "\"phase\":\"accepted\"",
        &format!(
            "\"phase\":\"semantic_error\",\"supersedes_receipt_sha256\":\"{}\"",
            line_sha256(old)
        ),
    )
}

/// A receipt in the importer's shape, for the case above.
pub(crate) fn receipt_line() -> String {
    format!(
        concat!(
            r#"{{"format":"sqlquack-syntax-receipt/1","case_id":"query.select-17","#,
            r#""sql_sha256":"ef83e1f6253a9a3e957db2ad55395a2ff7743a020a4f73c6085dbcbc5cb09ac7","#,
            r#""setup_sha256":"c5305748108cd15f217a3b64bd0dc9a891afc95159da14577e7ceb216bd1b9cc","#,
            r#""session_sha256":"126d70f8bbd834c0e341349f5804accba9c481e8c6acf3dfe1ac15b51414f21f","#,
            r#""session_parameters":{{"TIMEZONE":"UTC","USE_CACHED_RESULT":false,"#,
            r#""STATEMENT_TIMEOUT_IN_SECONDS":30}},"capture_method":"execute","#,
            r#""run_id":"run-1","query_id":"01b2-0000","warehouse_version":"10.40.100","#,
            r#""recorded_at":"2026-11-03T22:45:26.048112+00:00","outcome":"accepted","#,
            r#""errno":null,"sqlstate":null,"phase":"accepted","phase_basis":"ran","#,
            r#""raw_record_sha256":"{a}","run_audit_sha256":"{b}","packet_sha256":"{a}"}}"#,
        ),
        a = SHA_A,
        b = SHA_B
    )
}

/// Where a receipt line takes an `account_state` field.
pub(crate) const ACCOUNT_STATE_AT: &str = "\"phase_basis\":\"ran\"";

/// `ACCOUNT_STATE_AT` followed by one account parameter whose basis is `basis`.
pub(crate) fn account_state(basis: &str) -> String {
    format!(
        "{ACCOUNT_STATE_AT},\"account_state\":{{\"CORTEX_MODELS_ALLOWLIST\":\
         {{\"basis\":\"{basis}\",\"value\":\"NONE\"}}}}"
    )
}

/// `receipt_line()` as a failed DCM PLAN of its case, its inner error ending the whole message.
pub(crate) fn dcm_line(inner_message: &str, position: &str) -> String {
    let message: String = format!(
        "Error during DCM INIT_PLAN of DCM Project RUN_DB.PUBLIC.DP1: Compilation failed in file \
         sources/definitions/d1.sql:\\n{inner_message}"
    );
    receipt_line()
        .replace(
            "\"capture_method\":\"execute\"",
            "\"capture_method\":\"dcm_definition_file\"",
        )
        .replace(
            "\"errno\":null,\"sqlstate\":null,\"phase\":\"accepted\"",
            &format!(
                "\"errno\":1003,\"sqlstate\":\"42000\",\"phase\":\"syntax_error\",\
                 \"message\":\"{message}\",\"inner_message\":\"{inner_message}\",\
                 \"dcm_project\":\"RUN_DB.PUBLIC.DP1\",\
                 \"definition_file_sha256\":\
                 \"ef83e1f6253a9a3e957db2ad55395a2ff7743a020a4f73c6085dbcbc5cb09ac7\"{position}"
            ),
        )
}

/// A valid tree with one record of every kind.
pub(crate) fn valid_tree(root: &Path) {
    write(
        root,
        "sources.toml",
        &format!(
            "[[source]]\nid = \"doc.select\"\nurl = \"https://docs.example.com/select\"\n\
             title = \"SELECT\"\nfamily = \"query\"\nretrieved_utc = \"2026-11-03T20:00:00Z\"\n\
             page_sha256 = \"{SHA_A}\"\nsyntax_sha256 = \"{SHA_B}\"\nsyntax_blocks = 2\n\
             status = \"forms\"\ndecisions = [\"merged elided block\"]\n\
             [[source]]\nid = \"doc.billing\"\nurl = \"https://docs.example.com/billing\"\n\
             title = \"Billing\"\nfamily = \"\"\nretrieved_utc = \"2026-11-03T20:00:00Z\"\n\
             page_sha256 = \"{SHA_A}\"\nsyntax_sha256 = \"{SHA_B}\"\nsyntax_blocks = 0\n\
             status = \"out_of_scope\"\ndecisions = []\n"
        ),
    );
    write(
        root,
        "indexes.toml",
        &format!("[[index]]\nname = \"sql-reference_llms.txt\"\nsha256 = \"{SHA_A}\"\n"),
    );
    write(
        root,
        "aliases.toml",
        "[[alias]]\nfrom = \"sf.query.selects\"\nto = [\"sf.query.select\"]\nreason = \"page renamed\"\n\
         [[alias]]\nfrom = \"sf.functions.day\"\nto = [\"sf.functions.dayofmonth\", \"sf.functions.dayofweek\"]\n\
         reason = \"one call form merged two names\"\n",
    );
    for family in FAMILIES {
        write(root, &format!("catalogue/{family}.toml"), "");
        write(root, &format!("obligations/{family}.toml"), "");
        write(root, &format!("cases/{family}.jsonl"), "");
    }
    write(
        root,
        "catalogue/query.toml",
        concat!(
            "[[form]]\nid = \"sf.query.select\"\nfamily = \"query\"\nsource = \"doc.select\"\n",
            "anchor = \"Syntax\"\nkind = \"statement\"\n",
            "grammar = [\"SELECT [ DISTINCT | ALL ] <expr> [ , ... ]\", \"SELECT TOP <n> <expr>\"]\n",
            "grammar_sha256 = \"9fb5cff08b449c60f42d6905d1f3d2a7fde262f814861a1abd2a0375188d514b\"\n",
            "uses = [\"expr\"]\nstatus = \"catalogued\"\n",
            "[[form]]\nid = \"sf.query.select.define-view\"\nfamily = \"query\"\n",
            "source = \"doc.select\"\nanchor = \"DEFINE VIEW\"\nkind = \"prose\"\ngrammar = []\n",
            "grammar_sha256 = \"prose:DEFINE VIEW\"\nuses = []\nstatus = \"absent_from_source\"\n",
            "[[definition]]\nid = \"doc.select::expr\"\nsource = \"doc.select\"\nname = \"expr\"\n",
            "grammar = \"<expr> ::= <column> | <literal>\"\n",
        ),
    );
    write(
        root,
        "obligations/query.toml",
        concat!(
            "[[obligation]]\nid = \"sf.query.select.o001\"\nkind = \"positive\"\nrequired = true\n",
            "[[obligation]]\nid = \"sf.query.select.o002\"\nkind = \"interaction\"\nrequired = true\n",
            "pair = [\"DISTINCT\", \"TOP\"]\nanchor = \"Usage notes\"\n",
            "[[obligation]]\nid = \"sf.query.select.o003\"\nkind = \"nesting_depth\"\n",
            "required = false\ndepths = [1, 8]\n",
            "[[obligation]]\nid = \"sf.query.select.o004\"\nkind = \"boundary_above\"\n",
            "required = true\ntarget = \"TOP <n>\"\n",
        ),
    );
    write(root, "cases/query.jsonl", &format!("{}\n", case_line()));
    write(
        root,
        "limits.toml",
        &format!(
            "[[limit]]\nform = \"sf.query.select\"\nkind = \"boundary_above\"\n\
             [[suppress]]\nform = \"sf.query.select\"\nreason = \"r\"\n\
             [[reading]]\nform = \"sf.query.select\"\ngrammar_sha256 = \"{SHA_A}\"\n\
             grammar = [\"SELECT <expr>\"]\nreason = \"r\"\n"
        ),
    );
    write(
        root,
        "receipts/first.jsonl",
        &format!("{}\n", receipt_line()),
    );
    write(root, "receipts/first.report.json", "{}");
    write(
        root,
        "reviews/query.toml",
        &format!(
            "[[review]]\nid = \"query-1\"\nreviewer_lane = \"c\"\nfamily = \"query\"\n\
             catalogue_sha256 = \"{SHA_A}\"\nobligations_sha256 = \"{SHA_B}\"\n\
             parser_tree = \"0123456789abcdef0123456789abcdef01234567\"\n\
             reviewed_utc = \"2026-11-03T23:00:00Z\"\nverdict = \"approved\"\n\
             [[review.exclusion]]\nform = \"sf.query.select.define-view\"\n\
             reason = \"valid only inside project files\"\n"
        ),
    );
    write(
        root,
        "reviews/exclusions.toml",
        &format!(
            "[[review]]\nid = \"pages-1\"\nreviewer_lane = \"backend\"\nfamily = \"\"\n\
             sources_sha256 = \"{SHA_A}\"\n\
             parser_tree = \"0123456789abcdef0123456789abcdef01234567\"\n\
             reviewed_utc = \"2026-11-03T23:00:00Z\"\nverdict = \"approved\"\n\
             [[review.exclusion]]\nsource = \"doc.billing\"\n\
             reason = \"documents view columns, not syntax\"\n"
        ),
    );
}

/// Adds `doc.select-copy`, a page restating SELECT, and lists it on `sf.query.select`.
pub(crate) fn restated_page(root: &Path) {
    let sources = std::fs::read_to_string(root.join("sources.toml")).expect("read");
    write(
        root,
        "sources.toml",
        &format!(
            "{sources}[[source]]\nid = \"doc.select-copy\"\nurl = \"https://docs.example.com/c\"\n\
             title = \"SELECT, again\"\nfamily = \"query\"\nretrieved_utc = \"2026-11-03T20:00:00Z\"\n\
             page_sha256 = \"{SHA_A}\"\nsyntax_sha256 = \"{SHA_B}\"\nsyntax_blocks = 1\n\
             status = \"restated\"\ndecisions = []\n"
        ),
    );
    replace(
        root,
        "catalogue/query.toml",
        "source = \"doc.select\"\n",
        "source = \"doc.select\"\nalso_sources = [\"doc.select-copy\"]\n",
    );
}

/// The fixture receipt, superseding the line hashed as `supersedes` when there is one.
pub(crate) fn superseding_receipt(supersedes: Option<&str>) -> Receipt {
    let mut receipt: Receipt = serde_json::from_str(&receipt_line()).expect("receipt");
    receipt.supersedes_receipt_sha256 = supersedes.map(str::to_owned);
    receipt
}
