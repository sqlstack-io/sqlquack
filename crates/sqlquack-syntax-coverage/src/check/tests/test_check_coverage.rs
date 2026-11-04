use crate::check::main::check_coverage::check_coverage;
use crate::check::models::{FormState, Reason};
use crate::check::tests::helpers::{
    OTHER_TREE, SETUP, TREE, case_in_zone, case_line, drop_lines, exclusion_review, form,
    obligation, opaque, receipt_line, replace, rewrite_select_17, select_17, structural,
    verified_tree, write,
};
use crate::check::tests::test_types::CheckTestCase;
use crate::formats::main::load_coverage::load_coverage;

#[test]
fn given_a_changed_tree_when_checking_then_the_form_fails_closed_with_its_reason() {
    let test_cases = [
        CheckTestCase {
            description: "the unchanged tree verifies, so every fault below is the cause",
            mutate: |_| {},
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "a deleted receipt",
            mutate: |root| drop_lines(root, "receipts/run-1.jsonl", "\"select-17\""),
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::MissingReceipt),
            expected_problem: None,
        },
        CheckTestCase {
            description: "an obligation omitted after review, with the case citing it",
            mutate: |root| {
                replace(
                    root,
                    "obligations/query.toml",
                    &obligation("sf.query.select.o002", "negative_neighbour"),
                    "",
                );
                drop_lines(root, "cases/query.jsonl", "\"select-from\"");
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a catalogue form deleted after review, its obligation left behind",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    &form("sf.query.select.top", "SELECT TOP <n> <expr>"),
                    "",
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: Some(
                "obligation sf.query.select.top.o001: form sf.query.select.top is not catalogued",
            ),
        },
        CheckTestCase {
            description: "a receipt whose phase the parser contradicts",
            mutate: |root| {
                replace(
                    root,
                    "receipts/run-1.jsonl",
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"syntax_error\"",
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::Disagreement),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case whose SQL changed after its receipt",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &case_line("select-17", "sf.query.select.o001", "SELECT 18", &[SETUP]),
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReceipt),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case whose setup changed after its receipt",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &case_line("select-17", "sf.query.select.o001", "SELECT 17", &[]),
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReceipt),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case whose session changed after its receipt",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &case_in_zone(
                        "select-17",
                        "sf.query.select.o001",
                        "SELECT 17",
                        &[SETUP],
                        "Europe/London",
                    ),
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReceipt),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a receipt with neither a query id nor its hash",
            mutate: |root| {
                replace(
                    root,
                    "receipts/run-1.jsonl",
                    "\"query_id\":\"query-select-17\"",
                    "\"query_id\":null",
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReceipt),
            expected_problem: None,
        },
        CheckTestCase {
            description: "two admissible receipts of different phases",
            mutate: |root| {
                let other: String = receipt_line(&select_17(), "semantic_error");
                let receipts: String =
                    std::fs::read_to_string(root.join("receipts/run-1.jsonl")).expect("read");
                write(
                    root,
                    "receipts/run-1.jsonl",
                    &format!("{receipts}{other}\n"),
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::AmbiguousReceipts),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a parser tree other than the reviewed one",
            mutate: |_| {},
            parse: structural,
            parser_tree: Some(OTHER_TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "an uncommitted parser, which no review can match",
            mutate: |_| {},
            parse: structural,
            parser_tree: None,
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a parser that accepts only through a Command or Raw fallback",
            mutate: |_| {},
            parse: opaque,
            parser_tree: Some(TREE),
            form: "sf.query.select.top",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::OpaqueFallback),
            expected_problem: None,
        },
        CheckTestCase {
            description: "an exclusion in a review that requested changes",
            mutate: |root| {
                drop_lines(root, "receipts/run-1.jsonl", "\"select-top\"");
                exclusion_review(root, "changes_requested", TREE);
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select.top",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::MissingReceipt),
            expected_problem: None,
        },
        CheckTestCase {
            description: "an exclusion in an approved review of another parser tree",
            mutate: |root| {
                drop_lines(root, "receipts/run-1.jsonl", "\"select-top\"");
                exclusion_review(root, "approved", OTHER_TREE);
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select.top",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::MissingReceipt),
            expected_problem: None,
        },
        CheckTestCase {
            description: "an exclusion in a current approved review, the only way to exclude",
            mutate: |root| {
                drop_lines(root, "receipts/run-1.jsonl", "\"select-top\"");
                exclusion_review(root, "approved", TREE);
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select.top",
            expected_state: FormState::Excluded,
            expected_reason: Some(Reason::MissingReceipt),
            expected_problem: None,
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        verified_tree(directory.path());
        (test_case.mutate)(directory.path());
        let coverage = load_coverage(directory.path())
            .unwrap_or_else(|problems| panic!("{}: {problems:#?}", test_case.description));
        let check = check_coverage(&coverage, test_case.parser_tree, &test_case.parse);
        let form = check
            .forms
            .iter()
            .find(|form| form.id == test_case.form)
            .unwrap_or_else(|| panic!("{}: no form {}", test_case.description, test_case.form));
        assert_eq!(
            form.state, test_case.expected_state,
            "{}: {form:#?}",
            test_case.description
        );
        let reason_found: bool = test_case
            .expected_reason
            .map_or(form.reasons.is_empty(), |reason| {
                form.reasons.contains(&reason)
            });
        assert!(
            reason_found,
            "{}: {:?}",
            test_case.description, form.reasons
        );
        let problem_found: bool = test_case
            .expected_problem
            .map_or(check.problems.is_empty(), |problem| {
                check.problems.iter().any(|found| found.contains(problem))
            });
        assert!(
            problem_found,
            "{}: {:?}",
            test_case.description, check.problems
        );
    }
}
