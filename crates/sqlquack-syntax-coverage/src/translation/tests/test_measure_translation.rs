use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::check::tests::helpers::{sha256, verified_tree, write};
use crate::formats::main::load_coverage::load_coverage;
use crate::translation::main::load_translation_files::load_translation_files;
use crate::translation::main::measure_translation::{TranslationInputs, measure_translation};
use crate::translation::models::{FormState, TranslationReport};
use crate::translation::tests::test_types::TranslationTestCase;

const WAIT_REASON: &str = "DuckDB has no session to suspend";

/// A fake `sqlquack` that converts `SELECT 17` and runs `convert` for any other SQL.
fn fake_sqlquack(directory: &Path, convert: &str) -> std::path::PathBuf {
    let path = directory.join("sqlquack");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nsql=$(cat)\nschema=\nwhile [ $# -gt 0 ]; do [ \"$1\" = --schema ] && schema=$2; shift; done\n\
             if [ \"$sql\" = 'SELECT 17' ]; then echo '{{\"converted\":true,\"refusals\":[]}}'; exit 0; fi\n{convert}\n"
        ),
    )
    .expect("fake");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
    path
}

fn refused(category: &str, construct: &str) -> String {
    format!(
        "echo '{{\"converted\":false,\"refusals\":[{{\"category\":\"{category}\",\"construct\":\"{construct}\"}}]}}'; exit 1"
    )
}

fn no_by_design(_: &Path) {}

fn by_design_entry(directory: &Path) {
    write(
        directory,
        "by-design.toml",
        &format!(
            "[[by_design]]\nconstruct = \"fn:SYSTEM$WAIT\"\nreason = \"{WAIT_REASON}\"\nduckdb_note = \"none\"\n"
        ),
    );
}

fn signed_off(directory: &Path) {
    by_design_entry(directory);
    write(
        directory,
        "reviews/auditor-by-design.toml",
        &format!(
            "[[signoff]]\nconstruct = \"fn:SYSTEM$WAIT\"\nreason_sha256 = \"{}\"\nreviewed_utc = \"2026-11-04T10:00:00Z\"\n",
            sha256(WAIT_REASON)
        ),
    );
}

/// A sign-off of an earlier reason: the entry's reason has changed since the auditor read it.
fn stale_signoff(directory: &Path) {
    by_design_entry(directory);
    write(
        directory,
        "reviews/auditor-by-design.toml",
        &format!(
            "[[signoff]]\nconstruct = \"fn:SYSTEM$WAIT\"\nreason_sha256 = \"{}\"\nreviewed_utc = \"2026-11-04T10:00:00Z\"\n",
            sha256("an earlier reason")
        ),
    );
}

fn measure(
    root: &Path,
    by_design: &BTreeSet<String>,
    binary: &Path,
) -> Result<TranslationReport, Vec<String>> {
    let coverage = load_coverage(root).expect("coverage loads");
    let work = tempfile::tempdir().expect("work");
    let policy = root.join("policy.toml");
    measure_translation(&TranslationInputs {
        coverage: &coverage,
        root,
        by_design,
        binary,
        policy: &policy,
        work: work.path(),
        family: None,
        jobs: 2,
    })
}

#[test]
fn given_a_conversion_report_when_measuring_then_the_form_takes_its_state() {
    let unresolved: String = refused("unresolved name", "relation:T");
    let test_cases = [
        TranslationTestCase {
            description: "converted",
            convert: "echo '{\"converted\":true,\"refusals\":[]}'".to_owned(),
            census: "",
            by_design: no_by_design,
            expected_state: FormState::Translates,
            expected_refusals: &[],
            expected_measurable: 2,
        },
        TranslationTestCase {
            description: "only an unresolved name and no census: a syntax probe, outside the base",
            convert: unresolved.clone(),
            census: "",
            by_design: no_by_design,
            expected_state: FormState::NoRelationInScope,
            expected_refusals: &[],
            expected_measurable: 1,
        },
        TranslationTestCase {
            description: "an unresolved name even with its census schema",
            convert: unresolved.clone(),
            census: r#"{"case_id":"select-top","relations":[{"name":"t","columns":[{"name":"a","type":"NUMBER"},{"name":"\"b \"\"q\"","type":"TEXT"}]}]}"#,
            by_design: no_by_design,
            expected_state: FormState::NeedsSchema,
            expected_refusals: &[],
            expected_measurable: 1,
        },
        TranslationTestCase {
            description: "the census schema reaches convert with names resolved as Snowflake resolves them",
            convert: format!(
                "grep -q '\"T\":\\[\\[\"A\",\"NUMBER\"\\],\\[\"b \\\\\"q\",\"TEXT\"\\]\\]' \"$schema\" && {{ echo '{{\"converted\":true,\"refusals\":[]}}'; exit 0; }}\n{unresolved}"
            ),
            census: r#"{"case_id":"select-top","relations":[{"name":"t","columns":[{"name":"a","type":"NUMBER"},{"name":"\"b \"\"q\"","type":"TEXT"}]}]}"#,
            by_design: no_by_design,
            expected_state: FormState::Translates,
            expected_refusals: &[],
            expected_measurable: 2,
        },
        TranslationTestCase {
            description: "refused only for a signed-off by-design construct",
            convert: refused("unsupported construct", "fn:SYSTEM$WAIT"),
            census: "",
            by_design: signed_off,
            expected_state: FormState::ByDesign,
            expected_refusals: &[],
            expected_measurable: 2,
        },
        TranslationTestCase {
            description: "a by-design entry whose reason changed after sign-off counts as not yet translated",
            convert: refused("unsupported construct", "fn:SYSTEM$WAIT"),
            census: "",
            by_design: stale_signoff,
            expected_state: FormState::NotYetTranslated,
            expected_refusals: &["unsupported construct: fn:SYSTEM$WAIT"],
            expected_measurable: 2,
        },
        TranslationTestCase {
            description: "refused only for a value divergence the policy refuses",
            convert: refused("a known divergence the policy refuses", "divergence:x"),
            census: "",
            by_design: no_by_design,
            expected_state: FormState::DeclaredValueDivergence,
            expected_refusals: &[],
            expected_measurable: 2,
        },
        TranslationTestCase {
            description: "convert fails without a report: not translated",
            convert: "exit 3".to_owned(),
            census: "",
            by_design: no_by_design,
            expected_state: FormState::NotYetTranslated,
            expected_refusals: &["conversion did not finish"],
            expected_measurable: 2,
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("tempdir");
        let root = directory.path().join("syntax");
        verified_tree(&root);
        if !test_case.census.is_empty() {
            write(
                &root,
                "census/query.jsonl",
                &format!("{}\n", test_case.census),
            );
        }
        let translation = directory.path().join("translation");
        (test_case.by_design)(&translation);
        let by_design = load_translation_files(&translation).expect(test_case.description);
        let binary = fake_sqlquack(directory.path(), &test_case.convert);
        let report = measure(&root, &by_design, &binary).expect(test_case.description);
        let cases: Vec<&str> = report
            .cases
            .iter()
            .map(|case| case.case_id.as_str())
            .collect();
        assert_eq!(
            cases,
            ["select-17", "select-top"],
            "{}: only parsed cases",
            test_case.description
        );
        let state = |id: &str| {
            report
                .forms
                .iter()
                .find(|form| form.id == id)
                .expect(id)
                .clone()
        };
        let select = state("sf.query.select");
        assert_eq!(
            select.state,
            FormState::Translates,
            "{}",
            test_case.description
        );
        assert_eq!(
            select.binds, None,
            "{}: its setup is not empty",
            test_case.description
        );
        let top = state("sf.query.select.top");
        assert_eq!(
            top.state, test_case.expected_state,
            "{}",
            test_case.description
        );
        assert_eq!(
            top.refusals, test_case.expected_refusals,
            "{}",
            test_case.description
        );
        assert_eq!(
            report.total.measurable_forms, test_case.expected_measurable,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_census_entry_for_an_unknown_case_when_measuring_then_it_is_refused() {
    let directory = tempfile::tempdir().expect("tempdir");
    let root = directory.path().join("syntax");
    verified_tree(&root);
    write(
        &root,
        "census/query.jsonl",
        "{\"case_id\":\"no-such-case\",\"relations\":[{\"name\":\"T\",\"columns\":[{\"name\":\"A\",\"type\":\"NUMBER\"}]}]}\n",
    );
    let binary = fake_sqlquack(directory.path(), "exit 3");
    let problems = measure(&root, &BTreeSet::new(), &binary).expect_err("refused");
    assert!(
        problems
            .iter()
            .any(|problem| problem.contains("no-such-case does not exist")),
        "{problems:?}"
    );
}
