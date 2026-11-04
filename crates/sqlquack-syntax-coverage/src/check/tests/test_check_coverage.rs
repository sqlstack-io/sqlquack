use crate::check::main::check_coverage::check_coverage;
use crate::check::models::{FormState, Reason};
use crate::check::tests::helpers::{
    OTHER_TREE, SETUP, TREE, append_receipt, case_in_zone, case_line, drop_lines, exclusion_review,
    form, obligation, opaque, read, receipt_line, rejecting, replace, rewrite_select_17, select_17,
    select_17_setup_sha256, sha256, structural, verified_tree, with_fields, write,
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
            description: "a receipt of another capture method than its case's",
            mutate: |root| {
                replace(
                    root,
                    "receipts/run-1.jsonl",
                    "\"capture_method\":\"execute\"",
                    "\"capture_method\":\"compile_only_explain\"",
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
            description: "an inadmissible receipt of another phase does not make it ambiguous",
            mutate: |root| {
                let stale: String = receipt_line(&select_17(), "syntax_error")
                    .replace(&select_17_setup_sha256(), &"b".repeat(64));
                append_receipt(root, &stale);
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "a required obligation that no case cites",
            mutate: |root| drop_lines(root, "cases/query.jsonl", "\"select-17\""),
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::MissingCase),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a second case citing a satisfied obligation, without a receipt",
            mutate: |root| {
                let cases: String = read(root, "cases/query.jsonl");
                let second: String =
                    case_line("select-18", "sf.query.select.o001", "SELECT 18", &[]);
                write(root, "cases/query.jsonl", &format!("{cases}{second}\n"));
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::MissingReceipt),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a semantic refusal the parser rejects",
            mutate: |root| {
                replace(
                    root,
                    "receipts/run-1.jsonl",
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"semantic_error\"",
                )
            },
            parse: rejecting,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::Disagreement),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a runtime failure the parser rejects",
            mutate: |root| {
                replace(
                    root,
                    "receipts/run-1.jsonl",
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"runtime_error\"",
                )
            },
            parse: rejecting,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::Disagreement),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a form whose only obligation is optional",
            mutate: |root| {
                replace(
                    root,
                    "obligations/query.toml",
                    "id = \"sf.query.select.top.o001\"\nkind = \"positive\"\nrequired = true",
                    "id = \"sf.query.select.top.o001\"\nkind = \"positive\"\nrequired = false",
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select.top",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::NoObligations),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a superseded receipt, which never votes",
            mutate: |root| {
                let first: String = receipt_line(&select_17(), "accepted");
                let later: String = with_fields(
                    &receipt_line(&select_17(), "syntax_error"),
                    serde_json::json!({"supersedes_receipt_sha256": sha256(&first)}),
                );
                append_receipt(root, &later);
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::Disagreement),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a receipt recorded for another measure than syntax",
            mutate: |root| {
                let other: String = with_fields(
                    &receipt_line(&select_17(), "accepted"),
                    serde_json::json!({"measure": "d73_missing_object_with_setup"}),
                );
                drop_lines(root, "receipts/run-1.jsonl", "\"select-17\"");
                append_receipt(root, &other);
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReceipt),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case expected to be rejected that Snowflake accepted",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &with_fields(&select_17(), serde_json::json!({"expect": "reject"})),
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::UnexpectedOutcome),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case expected to be accepted that Snowflake rejected as syntax",
            mutate: |root| {
                let case: String =
                    case_line("select-from", "sf.query.select.o002", "SELECT FROM", &[]);
                replace(
                    root,
                    "cases/query.jsonl",
                    &case,
                    &with_fields(&case, serde_json::json!({"expect": "accept"})),
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::UnexpectedOutcome),
            expected_problem: None,
        },
        CheckTestCase {
            description: "an acceptance expected to fail at runtime that failed semantic analysis: stages never count",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &with_fields(
                        &select_17(),
                        serde_json::json!({"expect": "accept", "stage": "runtime"}),
                    ),
                );
                replace(
                    root,
                    "receipts/run-1.jsonl",
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"semantic_error\"",
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case expected to fail semantically or later that failed at runtime",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &with_fields(
                        &select_17(),
                        serde_json::json!({"expect": "accept", "stage": "semantic"}),
                    ),
                );
                replace(
                    root,
                    "receipts/run-1.jsonl",
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"runtime_error\"",
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "expectations met: a rejection expected and recorded, an acceptance too",
            mutate: |root| {
                let case: String =
                    case_line("select-from", "sf.query.select.o002", "SELECT FROM", &[]);
                replace(
                    root,
                    "cases/query.jsonl",
                    &case,
                    &with_fields(&case, serde_json::json!({"expect": "reject"})),
                );
                rewrite_select_17(
                    root,
                    &with_fields(&select_17(), serde_json::json!({"expect": "accept"})),
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "a review listing only another form leaves this one unreviewed",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "verdict = \"approved\"\n",
                    "verdict = \"approved\"\nforms = [\"sf.query.select.top\"]\n",
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::Unreviewed),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a current review listing the form reviews it",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "verdict = \"approved\"\n",
                    "verdict = \"approved\"\nforms = [\"sf.query.select\"]\n",
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "two current reviews listing the same form at the same binding",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "verdict = \"approved\"\n",
                    "verdict = \"approved\"\nforms = [\"sf.query.select\"]\n",
                );
                let first = read(root, "reviews/query.toml");
                write(
                    root,
                    "reviews/second.toml",
                    &first.replace("query-1", "query-2"),
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "a review listing a form that is not catalogued",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "verdict = \"approved\"\n",
                    "verdict = \"approved\"\nforms = [\"sf.query.select\", \"sf.query.nowhere\"]\n",
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: Some("review query-1: form sf.query.nowhere is not catalogued"),
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
