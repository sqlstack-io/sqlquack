//! `quoted_exclusions`: whether SQL text quotes a name in a star's EXCLUDE list.

/// Whether a name after EXCLUDE, alone or in its parenthesised list, is a quoted identifier.
pub fn quoted_exclusions(sql: &str) -> bool {
    let upper = sql.to_ascii_uppercase();
    upper.match_indices("EXCLUDE").any(|(start, keyword)| {
        let rest = sql[start + keyword.len()..].trim_start();
        match rest.strip_prefix('(') {
            Some(list) => list.split(')').next().unwrap_or_default().contains('"'),
            None => rest.starts_with('"'),
        }
    })
}
