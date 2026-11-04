use crate::check::models::FormState;
use crate::report::main::diff_snapshots::diff_snapshots;
use crate::report::models::{Snapshot, SnapshotDiff};
use crate::report::tests::helpers::{base, ids};
use crate::report::tests::test_types::DiffTestCase;

#[test]
fn given_a_later_snapshot_when_diffing_then_regressions_and_catalogue_changes_are_named() {
    let test_cases = [
        DiffTestCase {
            description: "only the parser tree moved",
            head: |head| Snapshot {
                parser_tree: None,
                ..head
            },
            expected_diff: SnapshotDiff::default(),
        },
        DiffTestCase {
            description: "a verified form went stale and an excluded one lost its review",
            head: |mut head| {
                head.forms
                    .insert("sf.query.select".to_owned(), FormState::Unverified);
                head.forms
                    .insert("sf.query.select.top".to_owned(), FormState::Unverified);
                head.satisfied_obligations.clear();
                head
            },
            expected_diff: SnapshotDiff {
                forms_lost: ids(&["sf.query.select", "sf.query.select.top"]),
                obligations_lost: ids(&["sf.query.select.o001"]),
                ..SnapshotDiff::default()
            },
        },
        DiffTestCase {
            description: "a form verified that was not, which is a gain and no regression",
            head: |mut head| {
                head.forms
                    .insert("sf.query.with".to_owned(), FormState::Verified);
                head.satisfied_obligations
                    .insert("sf.query.with.o001".to_owned());
                head
            },
            expected_diff: SnapshotDiff {
                forms_gained: ids(&["sf.query.with"]),
                obligations_gained: ids(&["sf.query.with.o001"]),
                ..SnapshotDiff::default()
            },
        },
        DiffTestCase {
            description: "obligations gained past 999 are listed by number, not as text",
            head: |mut head| {
                for number in ["o1000", "o999", "o100"] {
                    head.satisfied_obligations
                        .insert(format!("sf.query.with.{number}"));
                }
                head
            },
            expected_diff: SnapshotDiff {
                obligations_gained: ids(&[
                    "sf.query.with.o100",
                    "sf.query.with.o999",
                    "sf.query.with.o1000",
                ]),
                ..SnapshotDiff::default()
            },
        },
        DiffTestCase {
            description: "the catalogue changed: a file hash, a form added and one removed",
            head: |mut head| {
                head.catalogue_files
                    .insert("catalogue/query.toml".to_owned(), "b".repeat(64));
                head.catalogue_files
                    .insert("aliases.toml".to_owned(), "c".repeat(64));
                head.forms.remove("sf.query.with");
                head.forms
                    .insert("sf.query.values".to_owned(), FormState::Unverified);
                head
            },
            expected_diff: SnapshotDiff {
                catalogue_files_changed: ids(&["aliases.toml", "catalogue/query.toml"]),
                forms_added: ids(&["sf.query.values"]),
                forms_removed: ids(&["sf.query.with"]),
                ..SnapshotDiff::default()
            },
        },
        DiffTestCase {
            description: "a verified form removed from the catalogue is also lost",
            head: |mut head| {
                head.forms.remove("sf.query.select");
                head
            },
            expected_diff: SnapshotDiff {
                forms_removed: ids(&["sf.query.select"]),
                forms_lost: ids(&["sf.query.select"]),
                ..SnapshotDiff::default()
            },
        },
    ];
    for test_case in test_cases {
        let diff: SnapshotDiff = diff_snapshots(&base(), &(test_case.head)(base()));
        assert_eq!(diff, test_case.expected_diff, "{}", test_case.description);
    }
}
