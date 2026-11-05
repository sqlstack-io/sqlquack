//! Fixtures for the result-matching measure: a catalogue with two function forms and statements.

use std::collections::BTreeMap;
use std::path::Path;

use crate::check::tests::helpers::{sha256, verified_tree, write};
use crate::formats::main::load_coverage::load_coverage;
use crate::results::constants::STATEMENTS_FORMAT;
use crate::results::main::load_results_files::load_results_files;
use crate::results::main::measure_results::measure_results;
use crate::results::models::{
    RecordedStatement, ResultState, ResultsBaseline, ResultsReport, StatementOutcome,
    StatementsFile,
};

/// Links the plain select to its form and a window over IFF to the TOP form; columns name none.
pub(crate) const LINKS: &str = "[[link]]\nconstructs = [\"node:select\"]\nforms = [\"sf.query.select\"]\n\
    [[link]]\nconstructs = [\"fn:IFF\", \"node:window_function\"]\nforms = [\"sf.query.select.top\"]\n\
    [[no_form]]\nconstructs = [\"node:column\"]\nreason = \"part of other forms\"\n";
/// The reason of the fixture's one not-translatable group.
const LIST_REASON: &str = "DuckDB has no such thing";

fn call_form(id: &str, grammar: &str) -> String {
    format!(
        "[[form]]\nid = \"{id}\"\nfamily = \"functions\"\nsource = \"doc.select\"\n\
         anchor = \"Syntax\"\nkind = \"call\"\ngrammar = [\"{grammar}\"]\n\
         grammar_sha256 = \"{}\"\nuses = []\nstatus = \"catalogued\"\n",
        sha256(grammar)
    )
}

/// A verified query tree under `directory/syntax` with COUNT_IF and IFF call forms.
pub(crate) fn tree(directory: &Path) -> std::path::PathBuf {
    let root = directory.join("syntax");
    verified_tree(&root);
    write(
        &root,
        "catalogue/functions.toml",
        &[
            call_form("sf.functions.count-if", "COUNT_IF( <condition> )"),
            call_form("sf.functions.iff", "IFF( <condition> , <a> , <b> )"),
        ]
        .concat(),
    );
    root
}

/// The not-translatable group listing `form`.
pub(crate) fn list(form: &str) -> String {
    format!("[[group]]\nid = \"hosted\"\nreason = \"{LIST_REASON}\"\nforms = [\"{form}\"]\n")
}

/// The auditor's sign-off of the group `list(form)` writes.
pub(crate) fn list_signoff(form: &str) -> String {
    format!(
        "[[signoff]]\nsubject = \"not-translatable:hosted\"\nsha256 = \"{}\"\nscope = \"full\"\n\
         reviewed_utc = \"2026-11-05T10:00:00Z\"\n",
        sha256(&format!("{LIST_REASON}\n{form}"))
    )
}

/// One recorded statement using `constructs`.
pub(crate) fn statement(
    reference: &str,
    outcome: StatementOutcome,
    constructs: &[&str],
) -> RecordedStatement {
    RecordedStatement {
        reference: reference.to_owned(),
        hash: sha256(reference),
        outcome,
        codes: vec!["SQ0003".to_owned()],
        refused: Vec::new(),
        constructs: constructs
            .iter()
            .map(|construct| (*construct).to_owned())
            .collect(),
    }
}

/// The measure of `statements` over the fixture tree, with the given translation files.
pub(crate) fn measure(
    statements: Vec<RecordedStatement>,
    (links, not_translatable, review): (&str, &str, &str),
) -> Result<ResultsReport, Vec<String>> {
    let directory = tempfile::tempdir().expect("directory");
    let root = tree(directory.path());
    let translation = directory.path().join("translation");
    write(&translation, "form-links.toml", links);
    write(&translation, "not-translatable.toml", not_translatable);
    write(&translation, "reviews/auditor-results.toml", review);
    let coverage = load_coverage(&root).expect("coverage loads");
    let files = load_results_files(&translation, &coverage)?;
    measure_results(
        &coverage,
        &files,
        &StatementsFile {
            format: STATEMENTS_FORMAT.to_owned(),
            statements,
        },
    )
}

/// No translation file text, for a fixture that writes none.
pub(crate) fn nothing(_: &str) -> String {
    String::new()
}

/// A baseline holding one statement's outcome, or none.
pub(crate) fn baseline(outcome: Option<StatementOutcome>) -> ResultsBaseline {
    ResultsBaseline {
        statements: outcome
            .map(|outcome| BTreeMap::from([("f.toml#c".to_owned(), outcome)]))
            .unwrap_or_default(),
        ..ResultsBaseline::default()
    }
}

/// The state and linked statement count of form `id` in `statements`' measure.
pub(crate) fn form_state(
    statements: Vec<RecordedStatement>,
    id: &str,
) -> (ResultState, usize, usize) {
    let report = measure(statements, (LINKS, "", "")).expect("measures");
    let form = report
        .form_results
        .iter()
        .find(|form| form.id == id)
        .expect("form");
    (form.state, form.statements, report.forms.translatable)
}
