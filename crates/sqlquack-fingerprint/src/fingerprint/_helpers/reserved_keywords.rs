//! Per-dialect reserved keyword sets, read once from the keyword lists.

use std::collections::HashSet;
use std::sync::LazyLock;

pub(crate) static DUCKDB: LazyLock<HashSet<String>> =
    LazyLock::new(|| keyword_set(include_str!("../reserved_keywords/duckdb.txt")));
pub(crate) static POSTGRES: LazyLock<HashSet<String>> =
    LazyLock::new(|| keyword_set(include_str!("../reserved_keywords/postgres.txt")));
pub(crate) static SNOWFLAKE: LazyLock<HashSet<String>> =
    LazyLock::new(|| keyword_set(include_str!("../reserved_keywords/snowflake.txt")));
pub(crate) static GENERIC: LazyLock<HashSet<String>> =
    LazyLock::new(|| keyword_set(include_str!("../reserved_keywords/generic.txt")));

fn keyword_set(data: &str) -> HashSet<String> {
    data.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_ascii_uppercase)
        .collect()
}
