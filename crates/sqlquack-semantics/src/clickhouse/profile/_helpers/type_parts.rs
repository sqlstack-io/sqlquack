//! ClickHouse type spellings' parts: families, argument lists and decimal parameters.

use crate::clickhouse::profile::constants::{
    CLICKHOUSE_ALIASES, CLICKHOUSE_CASELESS_FAMILIES, CLICKHOUSE_EXACT_FAMILIES,
};

/// The family a type name or alias stands for, matched with case where ClickHouse does.
pub(crate) fn family(words: &str) -> Option<&'static str> {
    if let Some(exact) = CLICKHOUSE_EXACT_FAMILIES
        .iter()
        .find(|name| **name == words)
    {
        return Some(exact);
    }
    let upper: String = words.to_ascii_uppercase();
    CLICKHOUSE_CASELESS_FAMILIES
        .iter()
        .find(|name| name.to_ascii_uppercase() == upper)
        .copied()
        .or_else(|| {
            CLICKHOUSE_ALIASES
                .iter()
                .find(|(alias, _)| *alias == upper)
                .map(|(_, family)| *family)
        })
}

/// Type arguments as `toTypeName` writes them: trimmed, `, `-separated outside quoted strings.
pub(crate) fn listed(arguments: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut current: String = String::new();
    let mut quoted: bool = false;
    for character in arguments.chars() {
        if character == ',' && !quoted {
            parts.push(current.trim().to_owned());
            current.clear();
            continue;
        }
        quoted ^= character == '\'';
        current.push(character);
    }
    parts.push(current.trim().to_owned());
    parts.join(", ")
}

/// `Decimal(P)` or `Decimal(P, S)`, as `Decimal(P, S)`.
pub(crate) fn decimal(arguments: &str) -> Option<String> {
    let parts: Vec<&str> = arguments.split(',').map(str::trim).collect();
    let numbers: Vec<u32> = match parts
        .iter()
        .map(|part| part.parse::<u32>())
        .collect::<Result<Vec<u32>, _>>()
    {
        Ok(numbers) => numbers,
        Err(_not_a_number) => return None,
    };
    match numbers.as_slice() {
        [precision] => Some(format!("Decimal({precision}, 0)")),
        [precision, scale] => Some(format!("Decimal({precision}, {scale})")),
        _ => None,
    }
}
