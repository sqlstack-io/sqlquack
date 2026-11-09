//! Keywords a dialect folds only where they act as keywords.

use crate::dialects::main::profile::profile;
use crate::fingerprint::models::FingerprintDialect;

/// Whether the upper-cased word at `index` acts as one of `dialect`'s context-dependent keywords there.
pub(crate) fn folds_in_context(
    dialect: FingerprintDialect,
    uppers: &[String],
    index: usize,
) -> bool {
    profile(dialect).is_some_and(|profile| (profile.folds_in_context)(uppers, index))
}
