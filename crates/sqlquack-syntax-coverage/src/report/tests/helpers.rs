use std::collections::{BTreeMap, BTreeSet};

use crate::check::models::FormState;
use crate::formats::constants::SNAPSHOT_FORMAT;
use crate::report::models::Snapshot;

pub(crate) fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|id| (*id).to_owned()).collect()
}

/// One verified, one excluded and one unverified form, with one satisfied obligation.
pub(crate) fn base() -> Snapshot {
    Snapshot {
        format: SNAPSHOT_FORMAT.to_owned(),
        catalogue_commit: "e863d7de60ef88df1bce25a018f6592647b5729c".to_owned(),
        catalogue_files: BTreeMap::from([("catalogue/query.toml".to_owned(), "a".repeat(64))]),
        parser_tree: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
        forms: BTreeMap::from([
            ("sf.query.select".to_owned(), FormState::Verified),
            ("sf.query.select.top".to_owned(), FormState::Excluded),
            ("sf.query.with".to_owned(), FormState::Unverified),
        ]),
        satisfied_obligations: BTreeSet::from(["sf.query.select.o001".to_owned()]),
    }
}
