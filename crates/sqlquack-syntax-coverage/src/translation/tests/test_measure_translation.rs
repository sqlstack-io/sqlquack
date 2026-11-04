use std::collections::BTreeSet;

use crate::check::tests::helpers::{verified_tree, write};
use crate::translation::main::load_translation_files::load_translation_files;
use crate::translation::models::FormState;
use crate::translation::tests::helpers::{
    fake_sqlquack, measure, no_by_design, refused, signed_off, stale_signoff,
};
use crate::translation::tests::test_types::{CensusRefusalTestCase, TranslationTestCase};

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
            census: concat!(
                r#"{"case_id":"select-top","relations":[{"name":"t","columns":[{"name":"a","type":"NUMBER"},{"name":"\"b \"\"q\"","type":"TEXT"}]}]}"#,
                "\n"
            ),
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
            census: concat!(
                r#"{"case_id":"select-top","relations":[{"name":"t","columns":[{"name":"a","type":"NUMBER"},{"name":"\"b \"\"q\"","type":"TEXT"}]}]}"#,
                "\n"
            ),
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
        write(&root, "census/query.jsonl", test_case.census);
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
fn given_a_malformed_census_when_measuring_then_it_is_refused() {
    let test_cases = [
        CensusRefusalTestCase {
            description: "an entry for an unknown case",
            census: "{\"case_id\":\"no-such-case\",\"relations\":[{\"name\":\"T\",\"columns\":[{\"name\":\"A\",\"type\":\"NUMBER\"}]}]}\n",
            expected_problem: "case no-such-case does not exist",
        },
        CensusRefusalTestCase {
            description: "a case listed twice",
            census: "{\"case_id\":\"select-top\",\"relations\":[{\"name\":\"T\",\"columns\":[{\"name\":\"A\",\"type\":\"NUMBER\"}]}]}\n{\"case_id\":\"select-top\",\"relations\":[{\"name\":\"T\",\"columns\":[{\"name\":\"A\",\"type\":\"NUMBER\"}]}]}\n",
            expected_problem: "case select-top listed twice",
        },
        CensusRefusalTestCase {
            description: "a relation without columns",
            census: "{\"case_id\":\"select-top\",\"relations\":[{\"name\":\"T\",\"columns\":[]}]}\n",
            expected_problem: "a relation list or column list is empty",
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("tempdir");
        let root = directory.path().join("syntax");
        verified_tree(&root);
        write(&root, "census/query.jsonl", test_case.census);
        let binary = fake_sqlquack(directory.path(), "exit 3");
        let problems = measure(&root, &BTreeSet::new(), &binary).expect_err(test_case.description);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains(test_case.expected_problem)),
            "{}: {problems:?}",
            test_case.description
        );
    }
}
