use std::path::Path;

use crate::formats::models::Receipt;

/// A change to a valid coverage tree, and a fragment of the problem it must cause.
pub(crate) struct FormatFaultTestCase {
    pub(crate) description: &'static str,
    pub(crate) mutate: fn(&Path),
    pub(crate) expected_problem: &'static str,
}

/// How many records of each kind a valid tree loads.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LoadedCounts {
    pub(crate) sources: usize,
    pub(crate) indexes: usize,
    pub(crate) aliases: usize,
    pub(crate) forms: usize,
    pub(crate) definitions: usize,
    pub(crate) obligations: usize,
    pub(crate) cases: usize,
    pub(crate) receipts: usize,
    pub(crate) retired_receipts: usize,
    pub(crate) reviews: usize,
    pub(crate) file_hashes: usize,
}

/// A tree written into an empty directory, which must load.
pub(crate) struct LoadTestCase {
    pub(crate) description: &'static str,
    pub(crate) tree: fn(&Path),
    pub(crate) expected_counts: LoadedCounts,
}

/// The committed tree, which must load with every family present.
pub(crate) struct CommittedTestCase {
    pub(crate) description: &'static str,
    pub(crate) expected_families: &'static [&'static str],
}

/// Receipts with their line hashes, and what retiring them must leave.
pub(crate) struct RetireTestCase {
    pub(crate) description: &'static str,
    pub(crate) receipts: Vec<(Receipt, String)>,
    pub(crate) expected_kept: usize,
    pub(crate) expected_problem: Option<&'static str>,
}

/// A tree with a dialect file, which must load as that dialect.
pub(crate) struct DialectLoadTestCase {
    pub(crate) description: &'static str,
    pub(crate) tree: fn(&Path),
    pub(crate) expected_dialect: &'static str,
    pub(crate) expected_families: &'static [&'static str],
    pub(crate) expected_first_query_form: &'static str,
    pub(crate) expected_engine_versions: &'static [Option<&'static str>],
}

/// A focus file loaded against a tree, and how many entries it must keep.
pub(crate) struct FocusTestCase {
    pub(crate) description: &'static str,
    pub(crate) tree: fn(&Path),
    pub(crate) focus: &'static str,
    pub(crate) expected_entries: usize,
}

/// A focus file loaded against a tree, and a fragment of the problem it must cause.
pub(crate) struct FocusFaultTestCase {
    pub(crate) description: &'static str,
    pub(crate) tree: fn(&Path),
    pub(crate) focus: &'static str,
    pub(crate) expected_problem: &'static str,
}
