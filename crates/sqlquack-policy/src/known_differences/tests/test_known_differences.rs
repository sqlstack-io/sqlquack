use crate::known_differences::main::known_difference::known_difference;
use crate::known_differences::tests::helpers::{
    defaults_unlike_the_rule, entries_off_style, entries_unlike_policy, internal_names_in_text,
    notes_off_style,
};
use crate::known_differences::tests::test_types::{EntryCheckTestCase, LookupTestCase};

#[test]
fn given_a_name_when_looking_up_a_known_difference_then_only_a_divergence_name_finds_it() {
    let test_cases = [
        LookupTestCase {
            description: "a divergence name finds its entry",
            name: "hash",
            expected_title: Some("HASH gives a different value in DuckDB"),
        },
        LookupTestCase {
            description: "a construct key is not a name",
            name: "fn:HASH",
            expected_title: None,
        },
        LookupTestCase {
            description: "names are lower case",
            name: "HASH",
            expected_title: None,
        },
    ];
    for test_case in test_cases {
        let title = known_difference(test_case.name).map(|entry| entry.title);
        assert_eq!(title, test_case.expected_title, "{}", test_case.description);
    }
}

#[test]
fn given_the_repository_policy_when_checking_the_known_differences_then_each_matches_in_plain_words()
 {
    let test_cases = [
        EntryCheckTestCase {
            description: "every divergence has exactly one entry, failing as the policy says",
            findings: entries_unlike_policy,
            expected_findings: &[],
        },
        EntryCheckTestCase {
            description: "no text names a construct key, session name, flag or internal word",
            findings: internal_names_in_text,
            expected_findings: &[],
        },
        EntryCheckTestCase {
            description: "texts are short and each failure ends its wrong_when the same way",
            findings: entries_off_style,
            expected_findings: &[],
        },
        EntryCheckTestCase {
            description: "the evidence file refuses by default exactly the differences the default rule refuses",
            findings: defaults_unlike_the_rule,
            expected_findings: &[],
        },
        EntryCheckTestCase {
            description: "a diagnostic shows every entry's notes as whole, labelled sentences",
            findings: notes_off_style,
            expected_findings: &[],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            (test_case.findings)(),
            test_case.expected_findings,
            "{}",
            test_case.description
        );
    }
}
