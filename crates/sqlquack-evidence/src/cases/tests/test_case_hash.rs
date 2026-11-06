use crate::cases::tests::helpers::hashes;
use crate::cases::tests::test_types::HashTestCase;

#[test]
fn given_case_edit_when_hashing_then_only_result_changing_edits_change_the_hash() {
    let baseline = hashes("", "");
    let test_cases = [
        HashTestCase {
            description: "an unchanged file hashes the same",
            replace: "",
            with: "",
            expected_same_hash: true,
        },
        HashTestCase {
            description: "a new description keeps the hash",
            replace: "CONCAT with NULL and empty inputs.",
            with: "Another description.",
            expected_same_hash: true,
        },
        HashTestCase {
            description: "a new expression changes the hash",
            replace: "expression = \"CONCAT(a, b)\"",
            with: "expression = \"CONCAT(b, a)\"",
            expected_same_hash: false,
        },
        HashTestCase {
            description: "a changed input value changes the hash",
            replace: "[\"'gift'\", \"NULL\"]",
            with: "[\"'gift'\", \"'tag'\"]",
            expected_same_hash: false,
        },
        HashTestCase {
            description: "marking a case as typing evidence keeps the hash",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\nevidence = \"typing\"\nvalues = true",
            expected_same_hash: true,
        },
        HashTestCase {
            description: "recording no values changes the hash",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\nevidence = \"typing\"\nvalues = false",
            expected_same_hash: false,
        },
        HashTestCase {
            description: "a project function the case defines changes the hash",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\n\n[[case.function]]\nname = \"wrapped\"\narguments = [{ name = \"t\", type = \"VARCHAR\" }]\nreturns = \"VARCHAR\"\nbody = \"t\"",
            expected_same_hash: false,
        },
        HashTestCase {
            description: "an empty setup keeps the hash",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\nsetup = []",
            expected_same_hash: true,
        },
        HashTestCase {
            description: "a setup statement changes the hash",
            replace: "tags = [\"null\", \"empty\"]",
            with: "tags = [\"null\", \"empty\"]\nsetup = [\"SET x = 1\"]",
            expected_same_hash: false,
        },
        HashTestCase {
            description: "another session profile changes the hash",
            replace: "session_profile = \"utc-v1\"",
            with: "session_profile = \"utc-v2\"",
            expected_same_hash: false,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            hashes(test_case.replace, test_case.with)[0] == baseline[0],
            test_case.expected_same_hash,
            "{}",
            test_case.description
        );
    }
}
