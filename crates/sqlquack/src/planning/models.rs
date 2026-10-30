//! The model graph the plan reads.

use std::collections::BTreeSet;

use crate::census::models::RootKey;

/// Each file's own root refusals and the models it reads, by file index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PlanInput {
    /// The file's own root refusals: none for a file that converts.
    pub(crate) own: Vec<BTreeSet<RootKey>>,
    /// Models the file reads without known columns, which must convert to type it.
    pub(crate) needs: Vec<BTreeSet<usize>>,
    /// Every model the file reads.
    pub(crate) reads: Vec<BTreeSet<usize>>,
    /// Whether the file parsed.
    pub(crate) parsed: Vec<bool>,
    /// Whether the file may hold refusals that collecting did not reach.
    pub(crate) incomplete: Vec<bool>,
}

/// Which files are unit-testable and which strict under some supported constructs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Outcomes {
    /// Files whose own SQL converts, every model they need typed.
    pub(crate) unit: Vec<bool>,
    /// Unit-testable files every model upstream of which is strict too.
    pub(crate) strict: Vec<bool>,
}
