use crate::check::main::check_coverage::check_coverage;
use crate::check::tests::helpers::{TREE, structural, verified_tree};
use crate::formats::main::load_coverage::load_coverage;
use crate::report::main::summarize::summarize;
use crate::report::models::Summary;
use crate::report::tests::helpers::retire_select_top;
use crate::report::tests::test_types::SummarizeTestCase;

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
