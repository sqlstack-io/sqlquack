use std::collections::BTreeSet;

use crate::check::models::FormState;
use crate::report::main::diff_snapshots::diff_snapshots;
use crate::report::models::{ReceiptDisagreements, Snapshot, SnapshotDiff};
use crate::report::tests::helpers::{
    base, documented_false_accept, ids, listed_today, listing_k4, listing_retired,
};
use crate::report::tests::test_types::{BaseDiffTestCase, DiffTestCase, DisagreementTestCase};

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
            description: "a case Snowflake parsed and the base parser accepted is now rejected",
            head: |mut head| {
                head.parsed_cases.remove("sf.query.select.k1");
                head.refused_cases.insert("sf.query.select.k1".to_owned());
                head
            },
            expected_diff: SnapshotDiff {
                parsed_cases_refused: ids(&["sf.query.select.k1"]),
                unlisted_parsed_cases_refused: ids(&["sf.query.select.k1"]),
                ..SnapshotDiff::default()
            },
        },
        DiffTestCase {
            description: "a refused case now parsing, or one leaving the receipts, is no regression but resolves its entry",
            head: |mut head| {
                head.refused_cases.remove("sf.query.with.k2");
                head.parsed_cases.insert("sf.query.with.k2".to_owned());
                head.parsed_cases.remove("sf.query.select.k1");
                head
            },
            expected_diff: SnapshotDiff {
                disagreements_resolved: ids(&["sf.query.with.k2"]),
                ..SnapshotDiff::default()
            },
        },
        DiffTestCase {
            description: "a receipt the head newly contradicts, cited by a form or not, is a regression",
            head: |head| Snapshot {
                receipt_contradictions: Some(BTreeSet::from(["retired-1".to_owned()])),
                ..head
            },
            expected_diff: SnapshotDiff {
                receipts_contradicted: ids(&["retired-1"]),
                unlisted_receipts_contradicted: ids(&["retired-1"]),
                ..SnapshotDiff::default()
            },
        },
        DiffTestCase {
            description: "a documented case's receipt newly contradicted is listed apart",
            head: |mut head| {
                head.documented_cases
                    .insert("sf.query.select.k3".to_owned());
                head.receipt_contradictions =
                    Some(BTreeSet::from(["sf.query.select.k3".to_owned()]));
                head
            },
            expected_diff: SnapshotDiff {
                documented_receipts_contradicted: ids(&["sf.query.select.k3"]),
                unlisted_receipts_contradicted: ids(&["sf.query.select.k3"]),
                ..SnapshotDiff::default()
            },
        },
        DiffTestCase {
            description: "a receipt without SQL to judge is listed, not a regression",
            head: |head| Snapshot {
                unjudged_receipts: Some(BTreeSet::from(["retired-2".to_owned()])),
                ..head
            },
            expected_diff: SnapshotDiff {
                unjudged_receipts: ids(&["retired-2"]),
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
        let diff: SnapshotDiff = diff_snapshots(
            &base(),
            &(test_case.head)(base()),
            &listed_today(),
            Some(&listed_today()),
        );
        assert_eq!(diff, test_case.expected_diff, "{}", test_case.description);
    }
}

#[test]
fn given_a_base_without_receipt_verdicts_when_diffing_then_no_contradiction_is_new() {
    let test_cases = [
        BaseDiffTestCase {
            description: "every head contradiction is existing against a base from before receipt contradictions were tracked",
            base: |base| Snapshot {
                receipt_contradictions: None,
                ..base
            },
            head: |head| Snapshot {
                receipt_contradictions: Some(BTreeSet::from(["retired-1".to_owned()])),
                ..head
            },
            expected_diff: SnapshotDiff {
                receipts_still_contradicted: ids(&["retired-1"]),
                unlisted_receipts_contradicted: ids(&["retired-1"]),
                ..SnapshotDiff::default()
            },
        },
        BaseDiffTestCase {
            description: "a head from before receipt contradictions were tracked lists no receipts at all",
            base: |base| base,
            head: |head| Snapshot {
                receipt_contradictions: None,
                unjudged_receipts: None,
                ..head
            },
            expected_diff: SnapshotDiff::default(),
        },
    ];
    for test_case in test_cases {
        let diff: SnapshotDiff = diff_snapshots(
            &(test_case.base)(base()),
            &(test_case.head)(base()),
            &listed_today(),
            Some(&listed_today()),
        );
        assert_eq!(diff, test_case.expected_diff, "{}", test_case.description);
    }
}

#[test]
fn given_receipts_the_parser_contradicts_when_diffing_then_only_listed_ones_pass_and_the_list_only_shrinks()
 {
    let test_cases = [
        DisagreementTestCase {
            description: "a documented case accepted against its syntax error, before and now, unlisted",
            both: documented_false_accept,
            listed: listed_today,
            target: listed_today,
            expected_diff: SnapshotDiff {
                unlisted_syntax_errors_accepted: ids(&["sf.query.select.k4"]),
                ..SnapshotDiff::default()
            },
        },
        DisagreementTestCase {
            description: "the same case listed, here and on the target branch",
            both: documented_false_accept,
            listed: listing_k4,
            target: listing_k4,
            expected_diff: SnapshotDiff::default(),
        },
        DisagreementTestCase {
            description: "the same case listed here but not on the target branch",
            both: documented_false_accept,
            listed: listing_k4,
            target: listed_today,
            expected_diff: SnapshotDiff {
                disagreements_added: ids(&["sf.query.select.k4"]),
                ..SnapshotDiff::default()
            },
        },
        DisagreementTestCase {
            description: "a refused case no obligation judges any more, listed as another receipt",
            both: |mut snapshot| {
                snapshot.refused_cases.clear();
                snapshot.receipt_contradictions =
                    Some(BTreeSet::from(["sf.query.with.k2".to_owned()]));
                snapshot
            },
            listed: || ReceiptDisagreements {
                other_receipts_contradicted: BTreeSet::from(["sf.query.with.k2".to_owned()]),
                ..ReceiptDisagreements::default()
            },
            target: listed_today,
            expected_diff: SnapshotDiff {
                receipts_still_contradicted: ids(&["sf.query.with.k2"]),
                ..SnapshotDiff::default()
            },
        },
        DisagreementTestCase {
            description: "a listed case the parser now rejects, as Snowflake did",
            both: |snapshot| snapshot,
            listed: listing_k4,
            target: listing_k4,
            expected_diff: SnapshotDiff {
                disagreements_resolved: ids(&["sf.query.select.k4"]),
                ..SnapshotDiff::default()
            },
        },
        DisagreementTestCase {
            description: "a case Snowflake parsed, refused before and now, unlisted",
            both: |snapshot| snapshot,
            listed: ReceiptDisagreements::default,
            target: listed_today,
            expected_diff: SnapshotDiff {
                unlisted_parsed_cases_refused: ids(&["sf.query.with.k2"]),
                ..SnapshotDiff::default()
            },
        },
        DisagreementTestCase {
            description: "a receipt no obligation judges, contradicted before and now, unlisted",
            both: |snapshot| Snapshot {
                receipt_contradictions: Some(BTreeSet::from(["retired-1".to_owned()])),
                ..snapshot
            },
            listed: listed_today,
            target: listed_today,
            expected_diff: SnapshotDiff {
                unlisted_receipts_contradicted: ids(&["retired-1"]),
                receipts_still_contradicted: ids(&["retired-1"]),
                ..SnapshotDiff::default()
            },
        },
        DisagreementTestCase {
            description: "the same receipt listed",
            both: |snapshot| Snapshot {
                receipt_contradictions: Some(BTreeSet::from(["retired-1".to_owned()])),
                ..snapshot
            },
            listed: listing_retired,
            target: listing_retired,
            expected_diff: SnapshotDiff {
                receipts_still_contradicted: ids(&["retired-1"]),
                ..SnapshotDiff::default()
            },
        },
    ];
    for test_case in test_cases {
        let diff: SnapshotDiff = diff_snapshots(
            &(test_case.both)(base()),
            &(test_case.both)(base()),
            &(test_case.listed)(),
            Some(&(test_case.target)()),
        );
        assert_eq!(diff, test_case.expected_diff, "{}", test_case.description);
    }
}
