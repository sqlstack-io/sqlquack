use crate::check::main::check_coverage::check_coverage;
use crate::check::tests::helpers::{TREE, structural, verified_tree};
use crate::formats::main::load_coverage::load_coverage;
use crate::report::main::summarize::summarize;
use crate::report::models::Summary;
use crate::report::tests::helpers::retire_select_top;
use crate::report::tests::test_types::{SummarizeTestCase, SuppressionTestCase};

/// Two suppressed query negatives and one of a family with no catalogued form.
const SUPPRESSIONS: &str = r#"
[[suppress]]
form = "sf.query.select"
kind = "negative_neighbour"
reason = "r"

[[suppress]]
form = "sf.query.select.top"
kind = "negative_neighbour"
reason = "r"

[[suppress]]
form = "sf.ddl.create-table"
kind = "negative_neighbour"
reason = "r"
"#;

/// A retired catalogue id never counts, even when every obligation of it is satisfied.
#[test]
fn given_a_retired_form_that_would_verify_when_summarized_then_only_documented_forms_are_counted() {
    let test_cases = [
        SummarizeTestCase {
            description: "both catalogued forms verify and both count",
            mutate: |_| {},
            expected_forms: 2,
            expected_verified: 2,
            expected_absent_forms: 0,
        },
        SummarizeTestCase {
            description: "a retired form that would verify is left out of the total and the family",
            mutate: retire_select_top,
            expected_forms: 1,
            expected_verified: 1,
            expected_absent_forms: 1,
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path();
        verified_tree(root);
        (test_case.mutate)(root);
        let coverage = load_coverage(root, None).expect("load");
        let summary: Summary = summarize(&check_coverage(&coverage, Some(TREE), &structural));
        assert_eq!(
            (
                summary.total.forms,
                summary.total.verified,
                summary.absent_forms,
                summary.families["query"].tally.forms
            ),
            (
                test_case.expected_forms,
                test_case.expected_verified,
                test_case.expected_absent_forms,
                test_case.expected_forms
            ),
            "{}",
            test_case.description
        );
    }
}

/// Suppressed derived obligations are reported, in total and by family, and change no count.
#[test]
fn given_suppress_rows_when_summarized_then_they_are_reported_and_no_count_changes() {
    let test_cases = [
        SuppressionTestCase {
            description: "no suppress rows report nothing",
            suppressions: "",
            expected_suppressed: 0,
            expected_query_suppressed: 0,
        },
        SuppressionTestCase {
            description: "two query rows and one of a family with no catalogued form",
            suppressions: SUPPRESSIONS,
            expected_suppressed: 3,
            expected_query_suppressed: 2,
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        let root = directory.path();
        verified_tree(root);
        let unsuppressed: Summary = summarize(&check_coverage(
            &load_coverage(root, None).expect("load"),
            Some(TREE),
            &structural,
        ));
        let limits = root.join("limits.toml");
        let text: String = std::fs::read_to_string(&limits).expect("limits");
        std::fs::write(&limits, format!("{text}{}", test_case.suppressions)).expect("write limits");
        let summary: Summary = summarize(&check_coverage(
            &load_coverage(root, None).expect("load"),
            Some(TREE),
            &structural,
        ));
        assert_eq!(
            (
                summary.suppressed_obligations,
                summary.families["query"].suppressed_obligations,
                &summary.total,
            ),
            (
                test_case.expected_suppressed,
                test_case.expected_query_suppressed,
                &unsuppressed.total,
            ),
            "{}",
            test_case.description
        );
    }
}
