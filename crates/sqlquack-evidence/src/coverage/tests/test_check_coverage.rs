use crate::coverage::tests::helpers::{get_gaps, group_gaps, probed_shapes, unrecordable_split};
use crate::coverage::tests::test_types::{
    CoverageTestCase, GroupTestCase, ProbeTestCase, RecordedGet, RuleGaps, UnrecordableSplit,
    UnrecordableTestCase,
};
use crate::verify::models::Verdict;

const EMPTY_GAP: &str = "fn:GET[0] argument 1 text: empty";
const DOLLAR_GAP: &str = "fn:GET[0] argument 1 text: dollar_prefix";

#[test]
fn given_declared_domains_when_checking_coverage_then_uncovered_combinations_and_hazards_are_gaps()
{
    let test_cases = [
        CoverageTestCase {
            description: "a passing case with both text hazards covers the text key",
            key_families: &["text"],
            recorded: RecordedGet {
                cells: &["''", "'$.a'"],
                key_family: "text",
                key_source: "column:K",
                verdict: Verdict::Passed,
            },
            expected_gaps: RuleGaps {
                shape_gaps: vec![],
                undeclared_shapes: vec![],
                hazard_gaps: vec![],
            },
        },
        CoverageTestCase {
            description: "a declared integer key no case passes with is an uncovered combination",
            key_families: &["text", "integer"],
            recorded: RecordedGet {
                cells: &["''", "'$.a'"],
                key_family: "text",
                key_source: "column:K",
                verdict: Verdict::Passed,
            },
            expected_gaps: RuleGaps {
                shape_gaps: vec!["fn:GET(variant, integer:value)".to_owned()],
                undeclared_shapes: vec![],
                hazard_gaps: vec![],
            },
        },
        CoverageTestCase {
            description: "a key column never holding $-prefixed text leaves that hazard a gap",
            key_families: &["text"],
            recorded: RecordedGet {
                cells: &["''", "'a'"],
                key_family: "text",
                key_source: "column:K",
                verdict: Verdict::Passed,
            },
            expected_gaps: RuleGaps {
                shape_gaps: vec![],
                undeclared_shapes: vec![],
                hazard_gaps: vec![DOLLAR_GAP.to_owned()],
            },
        },
        CoverageTestCase {
            description: "a case without a recorded result covers nothing",
            key_families: &["text"],
            recorded: RecordedGet {
                cells: &["''", "'$.a'"],
                key_family: "text",
                key_source: "column:K",
                verdict: Verdict::Pending,
            },
            expected_gaps: RuleGaps {
                shape_gaps: vec!["fn:GET(variant, text:value)".to_owned()],
                undeclared_shapes: vec![],
                hazard_gaps: vec![EMPTY_GAP.to_owned(), DOLLAR_GAP.to_owned()],
            },
        },
        CoverageTestCase {
            description: "a literal key outside the declared value kind is accepted outside the domain",
            key_families: &["text"],
            recorded: RecordedGet {
                cells: &[],
                key_family: "text",
                key_source: "literal:'$.a'",
                verdict: Verdict::Passed,
            },
            expected_gaps: RuleGaps {
                shape_gaps: vec!["fn:GET(variant, text:value)".to_owned()],
                undeclared_shapes: vec!["fn:GET(variant:value, text:literal)".to_owned()],
                hazard_gaps: vec![EMPTY_GAP.to_owned(), DOLLAR_GAP.to_owned()],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            get_gaps(test_case.key_families, &test_case.recorded),
            test_case.expected_gaps,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_grouped_form_when_checking_coverage_then_unshown_row_set_hazards_are_gaps() {
    let test_cases = [
        GroupTestCase {
            description: "a group of NULLs and a group of two values show both hazards",
            groups: &["'a'", "'a'", "'b'", "'b'"],
            values: &["NULL", "NULL", "TRUE", "FALSE"],
            expected_hazard_gaps: vec![],
        },
        GroupTestCase {
            description: "groups each holding one value leave the mixed group a gap",
            groups: &["'a'", "'a'", "'b'"],
            values: &["NULL", "NULL", "TRUE"],
            expected_hazard_gaps: vec!["fn:ANY_VALUE[0] group: mixed".to_owned()],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            group_gaps(test_case.groups, test_case.values),
            test_case.expected_hazard_gaps,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_probe_when_checking_coverage_then_its_shapes_stay_outside_the_domain() {
    let test_cases = [ProbeTestCase {
        description: "a passing case of a probed shape is still accepted outside the domain",
        expected_outside: vec!["fn:GET(variant:value, integer:value)".to_owned()],
    }];
    for test_case in test_cases {
        assert_eq!(
            probed_shapes(),
            test_case.expected_outside,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_unrecordable_hazard_when_checking_coverage_then_it_is_reported_apart_from_the_gaps() {
    let test_cases = [UnrecordableTestCase {
        description: "an unrecordable hazard is no gap and is not counted among those needed",
        expected_split: UnrecordableSplit {
            hazard_gaps: vec![],
            unrecordable_hazards: vec!["fn:GET[0] argument 1 text: unconvertible".to_owned()],
            hazards: 2,
        },
    }];
    for test_case in test_cases {
        assert_eq!(
            unrecordable_split(),
            test_case.expected_split,
            "{}",
            test_case.description
        );
    }
}
