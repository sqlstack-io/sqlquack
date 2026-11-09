use crate::check::models::LocalFailure;
use crate::depth::main::compare_allowlist::compare_allowlist;
use crate::depth::models::Allowlist;
use crate::depth::tests::helpers::{entry, result, subjects};
use crate::depth::tests::test_types::AllowlistTestCase;
use crate::formats::models::LocalParser;

#[test]
fn given_an_allowlist_when_comparing_a_probe_run_then_unlisted_and_stale_entries_fail() {
    use LocalParser::{Postgres, Snowflake};
    let results = [
        result(Snowflake, "flat_plus", Some((10_000, LocalFailure::Crash))),
        result(Postgres, "case", Some((50, LocalFailure::Crash))),
        result(Postgres, "not", None),
    ];
    let test_cases = [
        AllowlistTestCase {
            description: "every failure listed at its depth",
            entries: vec![
                entry(Snowflake, "flat_plus", "gate", 10_000),
                entry(Postgres, "case", "gate", 50),
            ],
            expected_unlisted: vec![],
            expected_stale: vec![],
            expected_problems: vec![],
        },
        AllowlistTestCase {
            description: "a failure no entry lists",
            entries: vec![entry(Snowflake, "flat_plus", "gate", 10_000)],
            expected_unlisted: vec!["postgres case"],
            expected_stale: vec![],
            expected_problems: vec![],
        },
        AllowlistTestCase {
            description: "a failure listed at a shallower depth than it now fails at",
            entries: vec![
                entry(Snowflake, "flat_plus", "gate", 3_000),
                entry(Postgres, "case", "gate", 50),
            ],
            expected_unlisted: vec!["snowflake flat_plus"],
            expected_stale: vec![],
            expected_problems: vec![],
        },
        AllowlistTestCase {
            description: "a listed construct that now ends cleanly is stale",
            entries: vec![
                entry(Snowflake, "flat_plus", "gate", 10_000),
                entry(Postgres, "case", "gate", 50),
                entry(Postgres, "not", "gate", 1_000),
            ],
            expected_unlisted: vec![],
            expected_stale: vec!["postgres not"],
            expected_problems: vec![],
        },
        AllowlistTestCase {
            description: "another profile's entries neither excuse nor go stale",
            entries: vec![
                entry(Snowflake, "flat_plus", "dev", 10_000),
                entry(Postgres, "case", "dev", 50),
                entry(Postgres, "not", "dev", 1_000),
            ],
            expected_unlisted: vec!["snowflake flat_plus", "postgres case"],
            expected_stale: vec![],
            expected_problems: vec![],
        },
        AllowlistTestCase {
            description: "an entry the probe cannot check, and one listed twice",
            entries: vec![
                entry(Snowflake, "flat_plus", "gate", 10_000),
                entry(Postgres, "case", "gate", 50),
                entry(Postgres, "case", "gate", 50),
                entry(Postgres, "type_constructor", "dev", 50),
            ],
            expected_unlisted: vec![],
            expected_stale: vec![],
            expected_problems: vec!["postgres case", "postgres type_constructor"],
        },
    ];
    for test_case in test_cases {
        let report = compare_allowlist(
            &results,
            &Allowlist {
                known_failures: test_case.entries,
            },
            "gate",
        );
        assert_eq!(
            (
                subjects(&report.unlisted),
                subjects(&report.stale),
                subjects(&report.problems)
            ),
            (
                test_case.expected_unlisted,
                test_case.expected_stale,
                test_case.expected_problems
            ),
            "{}",
            test_case.description
        );
    }
}
