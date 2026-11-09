//! Constants of Databricks SQL's fingerprint profile.

/// Generic keywords that Databricks also takes as column and table names, so they fold only where they act as keywords.
pub(crate) const NAME_KEYWORDS: [&str; 3] = ["ANY", "ORDER", "SOME"];
/// `[word, next]`: `word` acts as a keyword before `next`. `ANY` and `SOME` fold before `(` as called words do.
pub(crate) const PAIRED_KEYWORDS: [[&str; 2]; 1] = [["ORDER", "BY"]];
