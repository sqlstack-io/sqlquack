//! The reserved keywords of Snowflake's fingerprint profile, read once from its keyword list.

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::dialects::main::keyword_set::keyword_set;

/// Snowflake's reserved keywords.
pub(crate) static RESERVED_KEYWORDS: LazyLock<HashSet<String>> =
    LazyLock::new(|| keyword_set(include_str!("../reserved_keywords.txt")));
