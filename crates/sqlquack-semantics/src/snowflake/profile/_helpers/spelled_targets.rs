//! Snowflake cast targets of type spellings, each spelling read once per thread.

use std::cell::RefCell;

use rustc_hash::FxHashMap;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::snowflake::profile::constants::SNOWFLAKE_KEPT_SPELLINGS;
use crate::snowflake::typing::main::cast_target::cast_target;
use crate::snowflake::typing::models::CastTarget;

thread_local! {
    /// What each spelling read on this thread denotes; none when it denotes no Snowflake type.
    static TARGETS: RefCell<FxHashMap<Box<str>, Option<CastTarget>>> =
        RefCell::new(FxHashMap::default());
}

/// The cast target `spelling` denotes, when it is a Snowflake type.
pub(crate) fn spelled_target(spelling: &str) -> Option<CastTarget> {
    if let Some(known) = TARGETS.with(|targets| targets.borrow().get(spelling).copied()) {
        return known;
    }
    let target: Option<CastTarget> = read_target(spelling);
    TARGETS.with(|targets| {
        let mut targets = targets.borrow_mut();
        if targets.len() >= SNOWFLAKE_KEPT_SPELLINGS {
            targets.clear();
        }
        targets.insert(Box::from(spelling), target);
    });
    target
}

/// The cast target `spelling` denotes, read afresh; none for a spelling Snowflake doesn't type.
fn read_target(spelling: &str) -> Option<CastTarget> {
    let target: CastTarget = match cast_target(spelling, &ProvenanceLog::default()) {
        Ok(target) => target,
        Err(_unknown) => return None,
    };
    Some(target)
}
