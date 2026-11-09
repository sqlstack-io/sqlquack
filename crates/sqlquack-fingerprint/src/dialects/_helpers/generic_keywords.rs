//! The keywords every dialect with a keyword list reserves, read once.

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::dialects::main::keyword_set::keyword_set;

/// The keywords reserved in every dialect with a keyword list, for SQL in no named dialect and SQLite.
pub(crate) static GENERIC_KEYWORDS: LazyLock<HashSet<String>> =
    LazyLock::new(|| keyword_set(include_str!("../reserved_keywords/generic.txt")));
