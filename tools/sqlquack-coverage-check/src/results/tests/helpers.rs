//! Fixtures for the result-matching measure: a catalogue of function forms and statements.

use std::collections::BTreeMap;
use std::path::Path;

use crate::check::tests::helpers::{sha256, verified_tree, write};
use crate::formats::main::load_coverage::load_coverage;
use crate::results::constants::{CALL, CALL_PREFIXES, FUNCTION_PREFIX, STATEMENTS_FORMAT};
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

/// Call shapes the fixture catalogue tells apart: plain, windowed, keyword and bare forms.
pub(crate) const SHAPE_FORMS: [(&str, &str); 8] = [
    ("sf.functions.sum", "SUM( <x> )"),
    (
        "sf.functions.sum-over",
        "SUM( <x> ) OVER ( [ PARTITION BY <p> ] )",
    ),
    (
        "sf.functions.regr-syy.x",
        "REGR_SYY(y, x) ( [ PARTITION BY <expr3> ] )",
    ),
    ("sf.functions.extract", "EXTRACT( <part> , <expr> )"),
    ("sf.functions.extract-from", "EXTRACT( <part> FROM <expr> )"),
    (
        "sf.functions.nth-value",
        "NTH_VALUE( <expr> , <n> ) [ FROM { FIRST | LAST } ] OVER ( ORDER BY <o> )",
    ),
    ("sf.functions.current-date", "CURRENT_DATE()"),
    ("sf.functions.current-date.syntax", "CURRENT_DATE"),
];

/// A verified query tree under `directory/syntax` with COUNT_IF, IFF and the call-shape forms.
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
        .into_iter()
        .chain(
            SHAPE_FORMS
                .iter()
                .map(|(id, grammar)| call_form(id, grammar)),
        )
        .collect::<String>(),
    );
    root
}

/// The not-translatable group listing `form`.
pub(crate) fn list(form: &str) -> String {
    format!("[[group]]\nid = \"hosted\"\nreason = \"{LIST_REASON}\"\nforms = [\"{form}\"]\n")
}

/// A reviewer's sign-off of the group `list(form)` writes.
pub(crate) fn list_signoff(form: &str) -> String {
    format!(
        "[[signoff]]\nsubject = \"not-translatable:hosted\"\nsha256 = \"{}\"\nscope = \"full\"\n\
         reviewed_utc = \"2026-11-05T10:00:00Z\"\n",
        sha256(&format!("{LIST_REASON}\n{form}"))
    )
}

/// One recorded statement using `constructs`, each function a plain call unless they give its shape.
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
        constructs: with_call_shapes(constructs),
    }
}

/// `constructs` with a plain call key for each function whose call shape they do not give.
fn with_call_shapes(constructs: &[&str]) -> Vec<String> {
    let shaped = |name: &str| {
        constructs.iter().any(|key| {
            CALL_PREFIXES
                .iter()
                .any(|prefix| key.strip_prefix(prefix) == Some(name))
        })
    };
    let plain_calls = constructs
        .iter()
        .filter_map(|key| key.strip_prefix(FUNCTION_PREFIX))
        .filter(|name| !shaped(name))
        .map(|name| format!("{CALL}{name}"));
    constructs
        .iter()
        .map(|key| (*key).to_owned())
        .chain(plain_calls)
        .collect()
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
    write(&translation, "reviews/results.toml", review);
    write(&translation, "obligations/functions.toml", "");
    std::fs::create_dir_all(directory.path().join("cases")).expect("cases root");
    let coverage = load_coverage(&root, None).expect("coverage loads");
    let files = load_results_files((&translation, &directory.path().join("cases")), &coverage)?;
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

/// The cited case of the COUNT_IF obligations, as a statement reference and as a case path.
pub(crate) const COUNT_IF_CASE: &str = "translation/functions/count-if.toml#t001";

/// One COUNT_IF obligation numbered `number`, with `fields` after its descriptive keys.
pub(crate) fn obligation(number: u32, fields: &str) -> String {
    format!(
        "[[obligation]]\nid = \"sf.functions.count-if.t{number:03}\"\nform = \"sf.functions.count-if\"\n\
         kind = \"type_combination\"\ntarget = \"BOOLEAN\"\nnote = \"n\"\ncatches = \"c\"\n\
         reading = \"r\"\n{fields}\n"
    )
}

/// The COUNT_IF case file under `parent`'s `cases/`, and an IFF one the obligations may wrongly cite.
pub(crate) fn count_if_cases(parent: &Path) {
    write(
        parent,
        "cases/translation/functions/count-if.toml",
        "construct = \"sf.functions.count-if\"\n",
    );
    write(
        parent,
        "cases/translation/functions/iff.toml",
        "construct = \"sf.functions.iff\"\n",
    );
}

/// The fields of a required obligation citing `reference` under `cases/`.
pub(crate) fn citing(reference: &str) -> String {
    format!("cases = [\"cases/{reference}\"]\nrequired = true")
}

/// The measure of `statements` over the fixture tree, with `obligations` as the functions file.
pub(crate) fn measure_obligations(
    statements: Vec<RecordedStatement>,
    (obligations, retired): (&str, &str),
) -> Result<ResultsReport, Vec<String>> {
    let directory = tempfile::tempdir().expect("directory");
    let root = tree(directory.path());
    let translation = directory.path().join("translation");
    write(&translation, "form-links.toml", LINKS);
    write(&translation, "obligations/functions.toml", obligations);
    write(&translation, "retired-obligations.toml", retired);
    count_if_cases(directory.path());
    let coverage = load_coverage(&root, None).expect("coverage loads");
    let files = load_results_files((&translation, &directory.path().join("cases")), &coverage)?;
    measure_results(
        &coverage,
        &files,
        &StatementsFile {
            format: STATEMENTS_FORMAT.to_owned(),
            statements,
        },
    )
}

/// Copies the files of the folder `name` under `from` to the same folder under `to`.
pub(crate) fn copy_folder(name: &str, from: &Path, to: &Path) {
    std::fs::create_dir_all(to.join(name)).expect("folder");
    for entry in std::fs::read_dir(from.join(name)).expect("folder") {
        let path = entry.expect("entry").path();
        std::fs::copy(&path, to.join(name).join(path.file_name().expect("name"))).expect("copy");
    }
}
