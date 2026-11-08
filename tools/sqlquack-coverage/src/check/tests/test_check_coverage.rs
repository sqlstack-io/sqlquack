use crate::check::main::check_coverage::check_coverage;
use crate::check::models::{FormState, Reason};
use crate::check::tests::helpers::{
    OTHER_TREE, SETUP, TREE, append_receipt, case_in_zone, case_line, crashing,
    crashing_on_select_from, drop_lines, exclusion_review, form, hanging, move_form, not_submitted,
    note_obligation, obligation, opaque, pin_definitions, pin_fingerprints, read, receipt_line,
    rejecting, replace, report_input, reshaped, retired_receipt, rewrite_select_17, select_17,
    select_17_setup_sha256, sha256, structural, unobservable, verified_tree, with_fields, write,
};
use crate::check::tests::test_types::{
    AccountStateTestCase, CheckTestCase, Parameters, ReceiptCheckTestCase,
};
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
            description: "an execute receipt never satisfies a case captured as a DCM definition file",
            mutate: |root| {
                replace(
                    root,
                    "cases/query.jsonl",
                    "\"capture\":\"execute\"",
                    "\"capture\":\"dcm_definition_file\"",
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
            description: "an unobservable obligation whose case otherwise agrees",
            mutate: |root| unobservable(root, "sf.query.select.o001", "select-17", "SELECT 17"),
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::UnverifiableInternalError),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a never-submitted obligation whose case otherwise agrees",
            mutate: |root| {
                not_submitted(root, "sf.query.select.o001", "select-17", "SELECT 17", "")
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::UnverifiableNotSubmitted),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a never-submitted entry whose case hash is stale",
            mutate: |root| {
                not_submitted(root, "sf.query.select.o001", "select-17", "SELECT 18", "")
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::UnverifiableNotSubmitted),
            expected_problem: Some("case select-17 is not a case with that sql_sha256"),
        },
        CheckTestCase {
            description: "a reading of a form that is not catalogued",
            mutate: |root| {
                replace(
                    root,
                    "limits.toml",
                    "form = \"sf.query.select.top\"",
                    "form = \"sf.query.tops\"",
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: Some("reading sf.query.tops: form is not catalogued"),
        },
        CheckTestCase {
            description: "an unobservable entry naming an undeclared obligation",
            mutate: |root| unobservable(root, "sf.query.select.o009", "select-17", "SELECT 17"),
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: Some("unobservable sf.query.select.o009: obligation is not declared"),
        },
        CheckTestCase {
            description: "an unobservable entry whose case hash is stale",
            mutate: |root| unobservable(root, "sf.query.select.o001", "select-17", "SELECT 18"),
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::UnverifiableInternalError),
            expected_problem: Some("case select-17 is not a case with that sql_sha256"),
        },
        CheckTestCase {
            description: "an unobservable entry naming a case that does not cite it",
            mutate: |root| unobservable(root, "sf.query.select.o001", "select-from", "SELECT FROM"),
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::UnverifiableInternalError),
            expected_problem: Some("case select-from is not a case with that sql_sha256"),
        },
        CheckTestCase {
            description: "a diagnostic naming a retired case, never resolved or scored",
            mutate: |root| {
                write(
                    root,
                    "unobservable/query.toml",
                    &format!(
                        "[[diagnostic]]\nobligation = \"sf.query.select.o001\"\n\
                         form = \"sf.query.select\"\nconstruct = \"c\"\nreason = \"603\"\n\
                         captured = [{{ case_id = \"retired-1\", sql_sha256 = \"{}\" }}]\n\
                         runs = [\"run-1\"]\nprobes = []\n",
                        sha256("SELECT 'gone'")
                    ),
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "a local parse that crashes",
            mutate: |_| {},
            parse: crashing,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::LocalParseFailure),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a local parse past its time bound",
            mutate: |_| {},
            parse: hanging,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::LocalParseFailure),
            expected_problem: None,
        },
        CheckTestCase {
            description: "an observe case without a receipt",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &with_fields(&select_17(), serde_json::json!({"expect": "observe"})),
                );
                drop_lines(root, "receipts/run-1.jsonl", "\"select-17\"");
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::AwaitingObservation),
            expected_problem: None,
        },
        CheckTestCase {
            description: "an observe case with a receipt, still never satisfied",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &with_fields(&select_17(), serde_json::json!({"expect": "observe"})),
                )
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::AwaitingObservation),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a terminates case Snowflake refused that the parser accepts",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &with_fields(&select_17(), serde_json::json!({"expect": "terminates"})),
                );
                replace(
                    root,
                    "receipts/run-1.jsonl",
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"syntax_error\"",
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
            description: "a terminates case whose local parse crashes",
            mutate: |root| {
                rewrite_select_17(
                    root,
                    &with_fields(&select_17(), serde_json::json!({"expect": "terminates"})),
                );
                replace(
                    root,
                    "receipts/run-1.jsonl",
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"syntax_error\"",
                );
            },
            parse: crashing,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::LocalParseFailure),
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

#[test]
fn given_a_review_with_fingerprints_when_the_parser_changes_then_only_changed_forms_are_stale() {
    let test_cases = [
        CheckTestCase {
            description: "another parser tree that parses the form's cases the same way",
            mutate: pin_fingerprints,
            parse: structural,
            parser_tree: Some(OTHER_TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "an uncommitted parser that parses the form's cases the same way",
            mutate: pin_fingerprints,
            parse: structural,
            parser_tree: None,
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "a parser that changes the tree of one of the form's cases",
            mutate: pin_fingerprints,
            parse: reshaped,
            parser_tree: Some(OTHER_TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "the same parser change leaves a form whose cases it does not touch current",
            mutate: pin_fingerprints,
            parse: reshaped,
            parser_tree: Some(OTHER_TREE),
            form: "sf.query.select.top",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "a parser that changes the verdict of one of the form's cases",
            mutate: pin_fingerprints,
            parse: crashing_on_select_from,
            parser_tree: Some(OTHER_TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case added to the form after review",
            mutate: |root| {
                pin_fingerprints(root);
                let added: String =
                    case_line("select-18", "sf.query.select.o001", "SELECT 18", &[]);
                let cases: String = read(root, "cases/query.jsonl");
                write(root, "cases/query.jsonl", &format!("{cases}{added}\n"));
                append_receipt(root, &receipt_line(&added, "accepted"));
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case removed from the form after review",
            mutate: |root| {
                pin_fingerprints(root);
                drop_lines(root, "cases/query.jsonl", "\"select-from\"");
                drop_lines(root, "receipts/run-1.jsonl", "\"select-from\"");
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a receipt reclassified from syntax to semantic after review, clearing a disagreement",
            mutate: |root| {
                let receipts: &str = "receipts/run-1.jsonl";
                replace(
                    root,
                    receipts,
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"syntax_error\"",
                );
                pin_fingerprints(root);
                replace(
                    root,
                    receipts,
                    "\"phase\":\"syntax_error\"",
                    "\"phase\":\"semantic_error\"",
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case's expected verdict declared after review",
            mutate: |root| {
                pin_fingerprints(root);
                rewrite_select_17(
                    root,
                    &with_fields(&select_17(), serde_json::json!({"expect": "accept"})),
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case's expected stage moved after review",
            mutate: |root| {
                let semantic: String = with_fields(
                    &select_17(),
                    serde_json::json!({"expect": "accept", "stage": "semantic"}),
                );
                rewrite_select_17(root, &semantic);
                pin_fingerprints(root);
                let runtime: String =
                    with_fields(&semantic, serde_json::json!({"stage": "runtime"}));
                replace(root, "cases/query.jsonl", &semantic, &runtime);
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
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
            "{}: {:?}",
            test_case.description, form.reasons
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
        assert!(
            check.problems.is_empty(),
            "{}: {:?}",
            test_case.description,
            check.problems
        );
    }
}

#[test]
fn given_a_review_binding_each_form_when_its_family_files_change_then_only_that_form_goes_stale() {
    let test_cases = [
        CheckTestCase {
            description: "a review binding each form, with nothing changed",
            mutate: pin_definitions,
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "another form's obligation edited in the same obligations file",
            mutate: |root| {
                pin_definitions(root);
                note_obligation(root, "sf.query.select.top.o001", "positive");
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "another form's entry edited in the same catalogue file",
            mutate: |root| {
                pin_definitions(root);
                move_form(root, "sf.query.select.top");
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "one of the form's own obligations edited after review",
            mutate: |root| {
                pin_definitions(root);
                note_obligation(root, "sf.query.select.o002", "negative_neighbour");
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "the form's own catalogue entry edited after review",
            mutate: |root| {
                pin_definitions(root);
                move_form(root, "sf.query.select");
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a case's SQL edited after review, with a bound receipt for the new SQL",
            mutate: |root| {
                pin_definitions(root);
                let edited: String =
                    case_line("select-17", "sf.query.select.o001", "SELECT 18", &[SETUP]);
                rewrite_select_17(root, &edited);
                append_receipt(root, &receipt_line(&edited, "accepted"));
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "a parser change to the tree of one of the form's cases",
            mutate: pin_definitions,
            parse: reshaped,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "the form's own reading edited after review",
            mutate: |root| {
                pin_definitions(root);
                replace(
                    root,
                    "limits.toml",
                    "read as one expression",
                    "read as a list",
                );
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
            expected_problem: None,
        },
        CheckTestCase {
            description: "another form's reading edited in the same limits file",
            mutate: |root| {
                pin_definitions(root);
                replace(root, "limits.toml", "read as one count", "read as a list");
            },
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Verified,
            expected_reason: None,
            expected_problem: None,
        },
        CheckTestCase {
            description: "a review binding the family's files still goes stale on another form's edit",
            mutate: |root| note_obligation(root, "sf.query.select.top.o001", "positive"),
            parse: structural,
            parser_tree: Some(TREE),
            form: "sf.query.select",
            expected_state: FormState::Unverified,
            expected_reason: Some(Reason::StaleReview),
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
            (form.state, form.reasons.clone()),
            (
                test_case.expected_state,
                test_case
                    .expected_reason
                    .into_iter()
                    .collect::<Vec<Reason>>()
            ),
            "{}",
            test_case.description
        );
        assert!(
            check.problems.is_empty(),
            "{}: {:?}",
            test_case.description,
            check.problems
        );
    }
}

#[test]
fn given_a_receipt_with_account_state_when_checking_then_only_its_case_carries_it() {
    let test_cases = [
        AccountStateTestCase {
            description: "a parameter with its value",
            account_state: ",\"account_state\":{\"CORTEX_MODELS_ALLOWLIST\":\
                            {\"value\":\"NONE\",\"basis\":\"set by sa-01\"}}",
            expected_cases: vec![
                ("select-17", vec![("CORTEX_MODELS_ALLOWLIST", Some("NONE"))]),
                ("select-from", vec![]),
                ("select-top", vec![]),
            ],
        },
        AccountStateTestCase {
            description: "a parameter whose value is null for want of a usable recorded_at",
            account_state: ",\"account_state\":{\"CORTEX_MODELS_ALLOWLIST\":\
                            {\"value\":null,\"basis\":\"no usable recorded_at\"}}",
            expected_cases: vec![
                ("select-17", vec![("CORTEX_MODELS_ALLOWLIST", None)]),
                ("select-from", vec![]),
                ("select-top", vec![]),
            ],
        },
    ];

    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        verified_tree(directory.path());
        replace(
            directory.path(),
            "receipts/run-1.jsonl",
            "\"phase_basis\":\"errno\"",
            &format!("\"phase_basis\":\"errno\"{}", test_case.account_state),
        );
        let coverage = load_coverage(directory.path())
            .unwrap_or_else(|problems| panic!("{}: {problems:#?}", test_case.description));
        let check = check_coverage(&coverage, Some(TREE), &structural);
        let cases: Vec<(&str, Parameters)> = check
            .forms
            .iter()
            .flat_map(|form| &form.obligations)
            .flat_map(|obligation| &obligation.cases)
            .map(|case| {
                let state = case
                    .account_state
                    .iter()
                    .map(|(name, setting)| (name.as_str(), setting.value.as_deref()))
                    .collect();
                (case.case_id.as_str(), state)
            })
            .collect();
        assert_eq!(cases, test_case.expected_cases, "{}", test_case.description);
        assert!(
            check
                .forms
                .iter()
                .filter(|form| form.family == "query")
                .all(|form| form.state == FormState::Verified),
            "{}: account state is provenance and changes no verdict",
            test_case.description
        );
    }
}

#[test]
fn given_receipts_no_form_cites_when_checking_then_each_is_judged_against_the_parser() {
    let test_cases = [
        ReceiptCheckTestCase {
            description: "a retired case Snowflake parsed, which the parser now rejects",
            mutate: |root| {
                retired_receipt(root, "SELECT 2", serde_json::json!({}));
                report_input(root, "SELECT 2");
            },
            parse: rejecting,
            expected_contradictions: vec!["retired-1"],
            expected_unjudged: Vec::new(),
        },
        ReceiptCheckTestCase {
            description: "a retired case Snowflake parsed, which the parser accepts",
            mutate: |root| {
                retired_receipt(root, "SELECT 2", serde_json::json!({}));
                report_input(root, "SELECT 2");
            },
            parse: structural,
            expected_contradictions: Vec::new(),
            expected_unjudged: Vec::new(),
        },
        ReceiptCheckTestCase {
            description: "a retired syntax error with errno 1003, which the parser accepts",
            mutate: |root| {
                retired_receipt(
                    root,
                    "SELECT 2",
                    serde_json::json!({"phase": "syntax_error", "outcome": "error", "errno": 1003}),
                );
                report_input(root, "SELECT 2");
            },
            parse: structural,
            expected_contradictions: vec!["retired-1"],
            expected_unjudged: Vec::new(),
        },
        ReceiptCheckTestCase {
            description: "a retired case whose SQL no case or report gives",
            mutate: |root| retired_receipt(root, "SELECT 2", serde_json::json!({})),
            parse: rejecting,
            expected_contradictions: Vec::new(),
            expected_unjudged: vec!["retired-1"],
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("tempdir");
        verified_tree(directory.path());
        (test_case.mutate)(directory.path());
        let coverage = load_coverage(directory.path()).expect("loads");
        let check = check_coverage(&coverage, Some(TREE), &test_case.parse);
        let contradicted: Vec<&str> = check
            .receipt_contradictions
            .iter()
            .map(String::as_str)
            .filter(|id| *id == "retired-1")
            .collect();
        let unjudged: Vec<&str> = check.unjudged_receipts.iter().map(String::as_str).collect();
        assert_eq!(
            (contradicted, unjudged),
            (
                test_case.expected_contradictions,
                test_case.expected_unjudged
            ),
            "{}",
            test_case.description
        );
    }
}
