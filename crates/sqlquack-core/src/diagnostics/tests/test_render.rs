use crate::diagnostics::main::diagnose::diagnose;
use crate::diagnostics::tests::helpers::{placed, rendered, unlocated, unparsed};
use crate::diagnostics::tests::test_types::{GuardTestCase, RenderTestCase};
use crate::refusals::models::RefusalCategory;

#[test]
fn given_refusals_when_rendered_then_the_source_line_is_underlined_at_the_span() {
    let call = "SELECT EDITDISTANCE(a, b) FROM orders";
    let tabbed = "SELECT\tEDITDISTANCE(a) FROM orders";
    let long = format!(
        "SELECT {}EDITDISTANCE(a) FROM orders WHERE {}TRUE",
        "a, ".repeat(100),
        "b AND ".repeat(30)
    );
    let multiline = "SELECT\n  EDITDISTANCE(a,\n  b) FROM orders";
    let test_cases = [
        RenderTestCase {
            description: "a call is underlined in full, with the code, title and construct",
            source: call.to_owned(),
            refusal: placed(call, (7, 25), RefusalCategory::MissingRule),
            expected_text: "error[SQ0002]: no verified rule converts this construct: `fn:EDITDISTANCE`\n \
                --> query.sql:1:8\n  |\n1 | SELECT EDITDISTANCE(a, b) FROM orders\n  |        \
                ^^^^^^^^^^^^^^^^^^ no rule converts this call\n"
                .to_owned(),
        },
        RenderTestCase {
            description: "a tab before the span widens the line and the underline alike",
            source: tabbed.to_owned(),
            refusal: placed(tabbed, (7, 22), RefusalCategory::UnresolvedName),
            expected_text: "error[SQ0004]: a name does not resolve: `fn:EDITDISTANCE`\n \
                --> query.sql:1:8\n  |\n1 | SELECT    EDITDISTANCE(a) FROM orders\n  |           \
                ^^^^^^^^^^^^^^^ no rule converts this call\n  |\n  = help: declare the relation and its \
                columns in the schema (`--schema` on the command line)\n"
                .to_owned(),
        },
        RenderTestCase {
            description: "a long line shows a window around the span, marked at both cut ends",
            source: long.clone(),
            refusal: placed(&long, (307, 322), RefusalCategory::MissingRule),
            expected_text: format!(
                "error[SQ0002]: no verified rule converts this construct: `fn:EDITDISTANCE`\n \
                 --> query.sql:1:308\n  |\n1 | …{}…\n  | {}{} no rule converts this call\n",
                &long[247..367],
                " ".repeat(61),
                "^".repeat(15)
            ),
        },
        RenderTestCase {
            description: "a span over several lines is underlined to the end of its first line",
            source: multiline.to_owned(),
            refusal: placed(multiline, (9, 29), RefusalCategory::MissingRule),
            expected_text: "error[SQ0002]: no verified rule converts this construct: `fn:EDITDISTANCE`\n \
                --> query.sql:2:3\n  |\n2 |   EDITDISTANCE(a,\n  |   ^^^^^^^^^^^^^^^ no rule \
                converts this call\n"
                .to_owned(),
        },
        RenderTestCase {
            description: "a parse failure points at where the parser stopped, with its message as the label",
            source: "SELECT (1".to_owned(),
            refusal: unparsed("SELECT (1"),
            expected_text: "error[SQ0001]: the SQL does not parse\n --> query.sql:1:9\n  |\n1 | \
                SELECT (1\n  |         ^ Expected RParen, got end of input ('') near [SELECT (1]\n"
                .to_owned(),
        },
        RenderTestCase {
            description: "a refusal without a place names the file and puts its reason in a note",
            source: call.to_owned(),
            refusal: unlocated(None),
            expected_text: "error[SQ0006]: conversion exceeded its budget\n --> query.sql\n  |\n  \
                = note: conversion took over 5 s\n  = help: split the statement into smaller statements\n"
                .to_owned(),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            rendered(&test_case.source, test_case.refusal),
            test_case.expected_text,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_guard_failures_when_diagnosed_then_each_guard_has_its_own_code_and_former_name() {
    let test_cases = [
        GuardTestCase {
            description: "nested parentheses beyond the limit",
            source: format!("SELECT {}1{}", "(".repeat(600), ")".repeat(600)),
            expected_code: ("SQ0103", Some("E_GUARD_NESTING_DEPTH_EXCEEDED")),
            expected_label: "value 513 exceeds configured limit 512",
        },
        GuardTestCase {
            description: "nested function calls beyond the limit",
            source: format!("SELECT {}1{}", "ABS(".repeat(400), ")".repeat(400)),
            expected_code: ("SQ0107", Some("E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED")),
            expected_label: "value 257 exceeds configured limit 256",
        },
        GuardTestCase {
            description: "an ordinary parse failure keeps the parse code and has no former name",
            source: "SELECT 'unterminated".to_owned(),
            expected_code: ("SQ0001", None),
            expected_label: "Unterminated string",
        },
    ];
    for test_case in test_cases {
        let diagnostics = diagnose(&[unparsed(&test_case.source)], &test_case.source);
        assert_eq!(
            (
                (diagnostics[0].code, diagnostics[0].alias),
                diagnostics[0].label.as_str()
            ),
            (test_case.expected_code, test_case.expected_label),
            "{}",
            test_case.description
        );
    }
}
