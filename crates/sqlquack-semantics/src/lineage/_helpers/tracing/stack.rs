//! Stack-safe recursion: the walks over a tree run on added stack segments when it nests deeply.

use crate::lineage::constants::{STACK_RED_ZONE, STACK_SEGMENT};

/// Runs one level of a recursive walk, on a new stack segment when the current one runs low.
pub(crate) fn deeper<R>(level: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(STACK_RED_ZONE, STACK_SEGMENT, level)
}
