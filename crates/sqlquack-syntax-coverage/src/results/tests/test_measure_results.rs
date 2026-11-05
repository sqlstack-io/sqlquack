use crate::results::models::{ResultState, StatementOutcome};
use crate::results::tests::helpers::{
    LINKS, SHAPE_FORMS, form_state, list, list_signoff, measure, nothing, statement,
};
use crate::results::tests::test_types::{
    CallShapeTestCase, FormStateTestCase, LinkTestCase, ListedMatchTestCase, RefusalTestCase,
    RefusedConstructTestCase,
};

const COUNT_IF: &str = "sf.functions.count-if";
const IFF: &str = "sf.functions.iff";
const COUNT_IF_CONSTRUCTS: &[&str] = &["fn:COUNT_IF", "node:column", "node:select"];

#[test]
fn given_linked_statements_when_measuring_then_the_form_takes_the_weakest_state() {
    let test_cases = [
        FormStateTestCase {
            description: "rows and a matching error both match",
            outcomes: &[StatementOutcome::Matching, StatementOutcome::MatchingError],
            list: nothing,
            review: nothing,
            expected_state: ResultState::Matching,
            expected_translatable: 11,
        },
        FormStateTestCase {
            description: "refused for one recorded type: not matching (D139a)",
            outcomes: &[
                StatementOutcome::Matching,
                StatementOutcome::Matching,
                StatementOutcome::Refused,
            ],
            list: nothing,
            review: nothing,
            expected_state: ResultState::Refused,
            expected_translatable: 11,
        },
        FormStateTestCase {
            description: "a guard raised where Snowflake returned rows",
            outcomes: &[
                StatementOutcome::Matching,
                StatementOutcome::DeclaredDivergence,
            ],
            list: nothing,
            review: nothing,
            expected_state: ResultState::Refused,
            expected_translatable: 11,
        },
        FormStateTestCase {
            description: "types only beside a match: the match decides",
            outcomes: &[StatementOutcome::Matching, StatementOutcome::ShapeOnly],
            list: nothing,
            review: nothing,
            expected_state: ResultState::Matching,
            expected_translatable: 11,
        },
        FormStateTestCase {
            description: "only a matching error: no rows compared, so not matching",
            outcomes: &[StatementOutcome::MatchingError],
            list: nothing,
            review: nothing,
            expected_state: ResultState::SyntaxOnly,
            expected_translatable: 11,
        },
        FormStateTestCase {
            description: "types only: syntax only, never matching",
            outcomes: &[StatementOutcome::ShapeOnly],
            list: nothing,
            review: nothing,
            expected_state: ResultState::SyntaxOnly,
            expected_translatable: 11,
        },
        FormStateTestCase {
            description: "converts but nothing recorded",
            outcomes: &[StatementOutcome::UnrecordedTranslates],
            list: nothing,
            review: nothing,
            expected_state: ResultState::SyntaxOnly,
            expected_translatable: 11,
        },
        FormStateTestCase {
            description: "a differing replay outranks a refusal",
            outcomes: &[
                StatementOutcome::Refused,
                StatementOutcome::Differs,
                StatementOutcome::Matching,
            ],
            list: nothing,
            review: nothing,
            expected_state: ResultState::Differs,
            expected_translatable: 11,
        },
        FormStateTestCase {
            description: "no recorded statement",
            outcomes: &[],
            list: nothing,
            review: nothing,
            expected_state: ResultState::Unmeasured,
            expected_translatable: 11,
        },
        FormStateTestCase {
            description: "listed and signed off: out of the denominator",
            outcomes: &[StatementOutcome::Refused],
            list,
            review: list_signoff,
            expected_state: ResultState::NotTranslatable,
            expected_translatable: 10,
        },
        FormStateTestCase {
            description: "listed without a sign-off: still counted",
            outcomes: &[],
            list,
            review: nothing,
            expected_state: ResultState::Unmeasured,
            expected_translatable: 11,
        },
    ];
    for test_case in test_cases {
        let statements = test_case
            .outcomes
            .iter()
            .enumerate()
            .map(|(index, outcome)| {
                statement(&format!("f.toml#c{index}"), *outcome, COUNT_IF_CONSTRUCTS)
            })
            .collect();
        let files = (
            LINKS,
            (test_case.list)(COUNT_IF),
            (test_case.review)(COUNT_IF),
        );

        let report =
            measure(statements, (files.0, &files.1, &files.2)).expect(test_case.description);

        let form = report
            .form_results
            .iter()
            .find(|form| form.id == COUNT_IF)
            .expect(test_case.description);
        assert_eq!(
            form.state, test_case.expected_state,
            "{}",
            test_case.description
        );
        assert_eq!(
            report.forms.translatable, test_case.expected_translatable,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_constructs_the_files_do_not_account_for_when_measuring_then_it_fails_closed() {
    let test_cases = [
        RefusalTestCase {
            description: "a node no link or no_form names",
            links: LINKS,
            constructs: &["node:select", "node:pivot"],
            expected_problem: "unmapped construct node:pivot in 1 statements",
        },
        RefusalTestCase {
            description: "a function the catalogue has no call form for",
            links: LINKS,
            constructs: &["fn:NOSUCH"],
            expected_problem: "unmapped construct fn:NOSUCH",
        },
        RefusalTestCase {
            description: "a link to a form the catalogue does not hold",
            links: "[[link]]\nconstructs = [\"node:select\"]\nforms = [\"sf.query.nosuch\"]\n",
            constructs: &["node:select"],
            expected_problem: "sf.query.nosuch is not a catalogued form",
        },
        RefusalTestCase {
            description: "a construct both linked and listed as naming no form",
            links: "[[link]]\nconstructs = [\"node:select\"]\nforms = [\"sf.query.select\"]\n\
                    [[no_form]]\nconstructs = [\"node:select\"]\nreason = \"r\"\n",
            constructs: &["node:select"],
            expected_problem: "node:select also links to forms",
        },
    ];
    for test_case in test_cases {
        let statements = vec![statement(
            "f.toml#c",
            StatementOutcome::Matching,
            test_case.constructs,
        )];

        let problems =
            measure(statements, (test_case.links, "", "")).expect_err(test_case.description);

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
fn given_a_signed_off_not_translatable_form_that_matches_when_measuring_then_it_fails() {
    let test_cases = [
        ListedMatchTestCase {
            description: "matching rows",
            outcome: StatementOutcome::Matching,
            expected_problem: "listed as not translatable, but f.toml#c translates",
        },
        ListedMatchTestCase {
            description: "a matching error",
            outcome: StatementOutcome::MatchingError,
            expected_problem: "listed as not translatable, but f.toml#c translates",
        },
    ];
    for test_case in test_cases {
        let statements = vec![statement(
            "f.toml#c",
            test_case.outcome,
            COUNT_IF_CONSTRUCTS,
        )];

        let problems = measure(
            statements,
            (LINKS, &list(COUNT_IF), &list_signoff(COUNT_IF)),
        )
        .expect_err(test_case.description);

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
fn given_a_two_construct_link_when_measuring_then_it_links_only_when_both_are_used() {
    let test_cases = [
        LinkTestCase {
            description: "IFF in a window: both IFF and the window link",
            refused_constructs: &["fn:IFF", "node:window_function"],
            expected_iff: (ResultState::Refused, 2),
            expected_top: ResultState::Refused,
            expected_blocked_forms: 2,
        },
        LinkTestCase {
            description: "IFF alone: only the function form links",
            refused_constructs: &["fn:IFF", "node:column"],
            expected_iff: (ResultState::Refused, 2),
            expected_top: ResultState::Unmeasured,
            expected_blocked_forms: 1,
        },
    ];
    for test_case in test_cases {
        let statements = vec![
            statement(
                "f.toml#plain",
                StatementOutcome::Matching,
                &["fn:IFF", "node:column"],
            ),
            statement(
                "f.toml#refused",
                StatementOutcome::Refused,
                test_case.refused_constructs,
            ),
            statement(
                "f.toml#column",
                StatementOutcome::Matching,
                &["node:column"],
            ),
            statement(
                "f.toml#pending",
                StatementOutcome::UnrecordedTranslates,
                &["fn:COUNT_IF"],
            ),
        ];

        let report = measure(statements, (LINKS, "", "")).expect(test_case.description);

        let state = |id: &str| {
            let form = report
                .form_results
                .iter()
                .find(|form| form.id == id)
                .expect(test_case.description);
            (form.state, form.statements)
        };
        assert_eq!(
            state(IFF),
            test_case.expected_iff,
            "{}",
            test_case.description
        );
        assert_eq!(
            state("sf.query.select.top").0,
            test_case.expected_top,
            "{}",
            test_case.description
        );
        assert_eq!(
            report.worklist[0].forms, test_case.expected_blocked_forms,
            "{}",
            test_case.description
        );
        assert_eq!(
            (
                report.statements.recorded,
                report.statements.matching,
                report.statements.unlinked
            ),
            (3, 2, 1),
            "{}",
            test_case.description
        );
        assert_eq!(
            report.unlinked_statements,
            ["f.toml#column"],
            "{}",
            test_case.description
        );
        assert_eq!(
            report.capture_statements,
            ["f.toml#pending"],
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_refusal_naming_one_construct_when_measuring_then_only_its_forms_are_refused() {
    let test_cases = [
        RefusedConstructTestCase {
            description: "names COUNT_IF: IFF keeps its own evidence",
            refused: &["fn:COUNT_IF"],
            expected_iff: ResultState::Matching,
        },
        RefusedConstructTestCase {
            description: "the whole statement",
            refused: &["statement"],
            expected_iff: ResultState::Refused,
        },
        RefusedConstructTestCase {
            description: "a construct naming no form",
            refused: &["node:column"],
            expected_iff: ResultState::Refused,
        },
    ];
    for test_case in test_cases {
        let mut refused = statement(
            "f.toml#refused",
            StatementOutcome::Refused,
            &["fn:COUNT_IF", "fn:IFF", "node:column"],
        );
        refused.refused = test_case
            .refused
            .iter()
            .map(|key| (*key).to_owned())
            .collect();
        let statements = vec![
            refused,
            statement("f.toml#iff", StatementOutcome::Matching, &["fn:IFF"]),
        ];

        let count_if = form_state(statements.clone(), COUNT_IF);
        let iff = form_state(statements, IFF);

        assert_eq!(
            count_if.0,
            ResultState::Refused,
            "{}",
            test_case.description
        );
        assert_eq!(iff.0, test_case.expected_iff, "{}", test_case.description);
    }
}

#[test]
fn given_call_shapes_when_measuring_then_each_call_credits_only_its_own_form() {
    let test_cases = [
        CallShapeTestCase {
            description: "a windowed SUM beside a plain call of another function credits only the OVER form",
            constructs: &["fn:SUM", "call_over:SUM", "fn:IFF", "node:window_function"],
            expected_forms: &["sf.functions.sum-over"],
        },
        CallShapeTestCase {
            description: "a plain and a windowed SUM credit both forms",
            constructs: &[
                "fn:SUM",
                "call:SUM",
                "call_over:SUM",
                "node:window_function",
            ],
            expected_forms: &["sf.functions.sum", "sf.functions.sum-over"],
        },
        CallShapeTestCase {
            description: "a plain REGR_SYY never credits the window form the page prints without OVER",
            constructs: &["fn:REGR_SYY"],
            expected_forms: &[],
        },
        CallShapeTestCase {
            description: "a comma EXTRACT call never credits the FROM form",
            constructs: &["fn:EXTRACT"],
            expected_forms: &["sf.functions.extract"],
        },
        CallShapeTestCase {
            description: "a bare CURRENT_DATE credits the bare form only",
            constructs: &["fn:CURRENT_DATE", "call_bare:CURRENT_DATE"],
            expected_forms: &["sf.functions.current-date.syntax"],
        },
        CallShapeTestCase {
            description: "CURRENT_DATE() credits the call form only",
            constructs: &["fn:CURRENT_DATE"],
            expected_forms: &["sf.functions.current-date"],
        },
    ];
    for test_case in test_cases {
        let statements = vec![statement(
            "f.toml#c",
            StatementOutcome::Matching,
            test_case.constructs,
        )];

        let report = measure(statements, (LINKS, "", "")).expect(test_case.description);

        let credited: Vec<&str> = SHAPE_FORMS
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| {
                report
                    .form_results
                    .iter()
                    .any(|form| form.id == *id && form.statements > 0)
            })
            .collect();
        assert_eq!(
            credited, test_case.expected_forms,
            "{}",
            test_case.description
        );
    }
}
