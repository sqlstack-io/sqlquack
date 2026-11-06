use std::process::ExitCode;

use crate::check::tests::helpers::write;
use crate::cli::_helpers::results_command::results;
use crate::cli::models::Common;
use crate::results::constants::STATEMENTS_FORMAT;
use crate::results::main::diff_results::diff_results;
use crate::results::models::{ObligationState, ResultState, ResultsBaseline, StatementOutcome};
use crate::results::tests::helpers::{
    COUNT_IF_CASE, LINKS, citing, count_if_cases, measure_obligations, obligation, statement, tree,
};
use crate::results::tests::test_types::{
    ObligationDiffTestCase, ObligationLoadTestCase, ObligationStateTestCase, ObligationsCliTestCase,
};

/// The statement under COUNT_IF's form that matches Snowflake, cited by no obligation.
const MATCHING: &str = "translation/functions/count-if.toml#t009";

#[test]
fn given_malformed_obligations_when_loading_then_each_is_a_load_error() {
    let cited = citing(COUNT_IF_CASE);
    let test_cases = [
        ObligationLoadTestCase {
            description: "a required obligation with no cases and no held reason",
            obligations: obligation(1, "cases = []\nrequired = true"),
            retired: "",
            expected_problem: "needs a non-empty held reason",
        },
        ObligationLoadTestCase {
            description: "a held obligation whose reason is blank",
            obligations: obligation(1, "cases = []\nrequired = true\nheld = \"  \""),
            retired: "",
            expected_problem: "needs a non-empty held reason",
        },
        ObligationLoadTestCase {
            description: "a held obligation that also cites a case",
            obligations: obligation(1, &format!("{cited}\nheld = \"parse gap\"")),
            retired: "",
            expected_problem: "with cases has neither held nor reason",
        },
        ObligationLoadTestCase {
            description: "held on an obligation that is not required",
            obligations: obligation(
                1,
                "cases = []\nrequired = false\nheld = \"x\"\nreason = \"y\"",
            ),
            retired: "",
            expected_problem: "held is for required obligations",
        },
        ObligationLoadTestCase {
            description: "a reason on a required obligation without cases",
            obligations: obligation(1, "cases = []\nrequired = true\nreason = \"x\""),
            retired: "",
            expected_problem: "a required obligation has no reason",
        },
        ObligationLoadTestCase {
            description: "an obligation that is not required, without a reason",
            obligations: obligation(1, "cases = []\nrequired = false"),
            retired: "",
            expected_problem: "needs a non-empty reason",
        },
        ObligationLoadTestCase {
            description: "an obligation that is not required but cites a case",
            obligations: obligation(
                1,
                &format!("cases = [\"cases/{COUNT_IF_CASE}\"]\nrequired = false\nreason = \"x\""),
            ),
            retired: "",
            expected_problem: "cites no cases",
        },
        ObligationLoadTestCase {
            description: "an unknown key",
            obligations: obligation(1, &format!("{cited}\nweight = 2")),
            retired: "",
            expected_problem: "unknown field `weight`",
        },
        ObligationLoadTestCase {
            description: "a form the catalogue does not hold",
            obligations: obligation(1, &cited).replace(
                "form = \"sf.functions.count-if\"",
                "form = \"sf.functions.count-if-not\"",
            ),
            retired: "",
            expected_problem: "not a catalogued form still in its source",
        },
        ObligationLoadTestCase {
            description: "an id declared twice",
            obligations: obligation(1, &cited) + &obligation(1, &cited),
            retired: "",
            expected_problem: "declared twice",
        },
        ObligationLoadTestCase {
            description: "an id that is not the form's",
            obligations: obligation(1, &cited).replace("count-if.t001", "iff.t001"),
            retired: "",
            expected_problem: "the id is not sf.functions.count-if.t<digits>",
        },
        ObligationLoadTestCase {
            description: "a boundary without a subkind",
            obligations: obligation(1, &cited).replace("type_combination", "boundary"),
            retired: "",
            expected_problem: "needs a subkind",
        },
        ObligationLoadTestCase {
            description: "a case under another family",
            obligations: obligation(
                1,
                "cases = [\"cases/translation/query/x.toml#t001\"]\nrequired = true",
            ),
            retired: "",
            expected_problem: "is not cases/translation/functions/<file>.toml#<id>",
        },
        ObligationLoadTestCase {
            description: "a retired id reused in obligations/",
            obligations: obligation(1, &cited),
            retired: "[[retired]]\nid = \"sf.functions.count-if.t001\"\nform = \"sf.functions.count-if\"\nreason = \"r\"\n",
            expected_problem: "the id is reused in obligations/",
        },
        ObligationLoadTestCase {
            description: "a retired id replaced by an obligation that does not exist",
            obligations: obligation(1, &cited),
            retired: "[[retired]]\nid = \"sf.functions.count-if.t002\"\nform = \"sf.functions.count-if\"\nreason = \"r\"\nreplaced_by = [\"sf.functions.count-if.t404\"]\n",
            expected_problem: "is not a live obligation",
        },
        ObligationLoadTestCase {
            description: "a retired id without a reason",
            obligations: obligation(1, &cited),
            retired: "[[retired]]\nid = \"sf.functions.count-if.t002\"\nform = \"sf.functions.count-if\"\nreason = \" \"\n",
            expected_problem: "needs its form and a reason",
        },
        ObligationLoadTestCase {
            description: "a cited case of another form",
            obligations: obligation(1, &citing("translation/functions/iff.toml#t001")),
            retired: "",
            expected_problem: "a case of sf.functions.iff",
        },
        ObligationLoadTestCase {
            description: "a cited case file that does not exist",
            obligations: obligation(1, &citing("translation/functions/count-if-gone.toml#t001")),
            retired: "",
            expected_problem: "count-if-gone.toml",
        },
        ObligationLoadTestCase {
            description: "a cited case no statement records",
            obligations: obligation(1, &citing("translation/functions/count-if.toml#t404")),
            retired: "",
            expected_problem: "no recorded statement is the case",
        },
    ];
    for test_case in test_cases {
        let problems = measure_obligations(
            vec![statement(
                COUNT_IF_CASE,
                StatementOutcome::Matching,
                &["fn:COUNT_IF"],
            )],
            (&test_case.obligations, test_case.retired),
        )
        .err()
        .unwrap_or_default();
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains(test_case.expected_problem)),
            "{}: {problems:?}",
            test_case.description
        );
    }
}

#[test]
fn given_obligations_when_measuring_then_a_form_matches_only_when_every_required_one_is_met() {
    let met = obligation(1, &citing(COUNT_IF_CASE));
    let test_cases = [
        ObligationStateTestCase {
            description: "the cited case matches",
            cited_outcome: StatementOutcome::Matching,
            obligations: met.clone(),
            expected_form: ResultState::Matching,
            expected_states: vec![ObligationState::Met],
        },
        ObligationStateTestCase {
            description: "the cited case only has its shape recorded, beside a matching statement",
            cited_outcome: StatementOutcome::ShapeOnly,
            obligations: met.clone(),
            expected_form: ResultState::UnmetObligations,
            expected_states: vec![ObligationState::Unmet],
        },
        ObligationStateTestCase {
            description: "the cited case has no Snowflake result",
            cited_outcome: StatementOutcome::UnrecordedTranslates,
            obligations: met.clone(),
            expected_form: ResultState::UnmetObligations,
            expected_states: vec![ObligationState::Unmet],
        },
        ObligationStateTestCase {
            description: "one of two cited cases only has its shape recorded",
            cited_outcome: StatementOutcome::ShapeOnly,
            obligations: obligation(
                1,
                &format!(
                    "cases = [\"cases/{MATCHING}\", \"cases/{COUNT_IF_CASE}\"]\nrequired = true"
                ),
            ),
            expected_form: ResultState::UnmetObligations,
            expected_states: vec![ObligationState::Unmet],
        },
        ObligationStateTestCase {
            description: "no obligation declared: matching rows alone do not match the form",
            cited_outcome: StatementOutcome::Matching,
            obligations: String::new(),
            expected_form: ResultState::NoObligations,
            expected_states: vec![],
        },
        ObligationStateTestCase {
            description: "only an unmeasured obligation: no required one, so the form cannot match",
            cited_outcome: StatementOutcome::Matching,
            obligations: obligation(
                3,
                "cases = []\nrequired = false\nreason = \"row order unspecified\"",
            ),
            expected_form: ResultState::NoObligations,
            expected_states: vec![ObligationState::Unmeasured],
        },
        ObligationStateTestCase {
            description: "a held obligation is never met, though every statement matches",
            cited_outcome: StatementOutcome::Matching,
            obligations: met.clone()
                + &obligation(2, "cases = []\nrequired = true\nheld = \"waits on #118\""),
            expected_form: ResultState::UnmetObligations,
            expected_states: vec![ObligationState::Met, ObligationState::Held],
        },
        ObligationStateTestCase {
            description: "an unmeasured obligation is never met and does not block the form",
            cited_outcome: StatementOutcome::Matching,
            obligations: met
                + &obligation(
                    3,
                    "cases = []\nrequired = false\nreason = \"row order unspecified\"",
                ),
            expected_form: ResultState::Matching,
            expected_states: vec![ObligationState::Met, ObligationState::Unmeasured],
        },
    ];
    for test_case in test_cases {
        let report = measure_obligations(
            vec![
                statement(MATCHING, StatementOutcome::Matching, &["fn:COUNT_IF"]),
                statement(COUNT_IF_CASE, test_case.cited_outcome, &["fn:COUNT_IF"]),
            ],
            (&test_case.obligations, ""),
        )
        .expect("measures");
        let form = report
            .form_results
            .iter()
            .find(|form| form.id == "sf.functions.count-if")
            .expect("form");
        let states: Vec<ObligationState> = report.obligation_states.values().copied().collect();
        assert_eq!(
            (form.state, states, report.obligations.met),
            (
                test_case.expected_form,
                test_case.expected_states.clone(),
                test_case
                    .expected_states
                    .iter()
                    .filter(|state| **state == ObligationState::Met)
                    .count()
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_obligations_when_running_the_results_command_then_the_json_reports_each_bucket() {
    let test_cases = [
        ObligationsCliTestCase {
            description: "a held and an unmeasured obligation land in their own buckets",
            obligations: Some(
                obligation(1, &citing(COUNT_IF_CASE))
                    + &obligation(2, "cases = []\nrequired = true\nheld = \"parse gap\"")
                    + &obligation(3, "cases = []\nrequired = false\nreason = \"unspecified\""),
            ),
            expected_exit: ExitCode::SUCCESS,
            expected_buckets: Some((1, 1, 1, 3)),
        },
        ObligationsCliTestCase {
            description: "no obligations directory fails the command, not zero obligations",
            obligations: None,
            expected_exit: ExitCode::FAILURE,
            expected_buckets: None,
        },
        ObligationsCliTestCase {
            description: "a required obligation without cases or held fails the command",
            obligations: Some(obligation(1, "cases = []\nrequired = true")),
            expected_exit: ExitCode::FAILURE,
            expected_buckets: None,
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("directory");
        let root = tree(&directory.path().join("coverage"));
        let translation = directory.path().join("coverage/translation");
        write(&translation, "form-links.toml", LINKS);
        test_case
            .obligations
            .iter()
            .for_each(|obligations| write(&translation, "obligations/functions.toml", obligations));
        count_if_cases(directory.path());
        let statements = serde_json::json!({
            "format": STATEMENTS_FORMAT,
            "statements": [
                statement(MATCHING, StatementOutcome::Matching, &["fn:COUNT_IF"]),
                statement(COUNT_IF_CASE, StatementOutcome::Matching, &["fn:COUNT_IF"]),
            ],
        });
        write(directory.path(), "statements.json", &statements.to_string());
        let report_path = directory.path().join("report.json");
        let exit = results(
            &directory.path().join("statements.json"),
            (true, Some(&report_path), None),
            &Common { root, json: true },
        )
        .unwrap_or_else(|exit| exit);
        let buckets = std::fs::read_to_string(&report_path).ok().map(|text| {
            let report: serde_json::Value = serde_json::from_str(&text).expect("report JSON");
            let baseline: ResultsBaseline = serde_json::from_str(
                &std::fs::read_to_string(translation.join("results-baseline.json"))
                    .expect("baseline"),
            )
            .expect("baseline JSON");
            let count = |key: &str| report["obligations"][key].as_u64().unwrap_or(u64::MAX);
            (
                count("met"),
                count("held"),
                count("unmeasured"),
                u64::try_from(baseline.obligations.len()).unwrap_or(u64::MAX),
            )
        });
        assert_eq!(
            (exit, buckets),
            (test_case.expected_exit, test_case.expected_buckets),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_met_obligation_when_it_stops_being_met_then_the_diff_regresses() {
    let id = "sf.functions.count-if.t001";
    let baseline = |state: Option<ObligationState>, retired: bool| ResultsBaseline {
        obligations: state
            .map(|state| (id.to_owned(), state))
            .into_iter()
            .collect(),
        retired: retired.then(|| id.to_owned()).into_iter().collect(),
        ..ResultsBaseline::default()
    };
    let test_cases = [
        ObligationDiffTestCase {
            description: "met to unmet regresses",
            before: ObligationState::Met,
            after: Some(ObligationState::Unmet),
            retired: false,
            expected_regressions: 1,
        },
        ObligationDiffTestCase {
            description: "met to held regresses",
            before: ObligationState::Met,
            after: Some(ObligationState::Held),
            retired: false,
            expected_regressions: 1,
        },
        ObligationDiffTestCase {
            description: "unmet to met is an improvement",
            before: ObligationState::Unmet,
            after: Some(ObligationState::Met),
            retired: false,
            expected_regressions: 0,
        },
        ObligationDiffTestCase {
            description: "held to unmet is a change, not a regression",
            before: ObligationState::Held,
            after: Some(ObligationState::Unmet),
            retired: false,
            expected_regressions: 0,
        },
        ObligationDiffTestCase {
            description: "an unmet obligation deleted without a tombstone regresses",
            before: ObligationState::Unmet,
            after: None,
            retired: false,
            expected_regressions: 1,
        },
        ObligationDiffTestCase {
            description: "a met obligation retired in retired-obligations.toml is a change",
            before: ObligationState::Met,
            after: None,
            retired: true,
            expected_regressions: 0,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            diff_results(
                &baseline(Some(test_case.before), false),
                &baseline(test_case.after, test_case.retired),
            )
            .regressions
            .len(),
            test_case.expected_regressions,
            "{}",
            test_case.description
        );
    }
}
