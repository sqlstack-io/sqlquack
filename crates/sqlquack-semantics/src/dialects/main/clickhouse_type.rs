//! `clickhouse_type`: a type spelling as ClickHouse 26.1 names the type it denotes.

use crate::dialects::_helpers::clickhouse_types::{decimal, family, listed};
use crate::dialects::constants::{
    CLICKHOUSE_DECIMAL_DEFAULT, CLICKHOUSE_DECIMAL_WIDTHS, CLICKHOUSE_NUMBER_FAMILIES,
    CLICKHOUSE_WRAPPERS,
};

/// The type `spelling` denotes, as `toTypeName` spells it; `None` for a spelling it does not know.
pub fn clickhouse_type(spelling: &str) -> Option<String> {
    let text: &str = spelling.trim();
    let (base, arguments): (&str, Option<&str>) = match text.find('(') {
        Some(open) => (
            text[..open].trim(),
            Some(text[open + 1..].strip_suffix(')')?),
        ),
        None => (text, None),
    };
    let words: String = base.split_whitespace().collect::<Vec<&str>>().join(" ");
    if let Some(wrapper) = CLICKHOUSE_WRAPPERS
        .iter()
        .find(|wrapper| **wrapper == words)
    {
        return Some(format!("{wrapper}({})", clickhouse_type(arguments?)?));
    }
    let family: &str = family(&words)?;
    match (family, arguments) {
        ("String", _) => Some(family.to_owned()),
        ("Decimal", None) => Some(CLICKHOUSE_DECIMAL_DEFAULT.to_owned()),
        ("Decimal", Some(arguments)) => decimal(arguments),
        ("FixedString", None) => None,
        (_, None) => Some(family.to_owned()),
        (_, Some(_))
            if CLICKHOUSE_NUMBER_FAMILIES
                .iter()
                .any(|prefix| family.starts_with(prefix)) =>
        {
            Some(family.to_owned())
        }
        (_, Some(arguments)) => {
            let width: Option<&(&str, u32)> = CLICKHOUSE_DECIMAL_WIDTHS
                .iter()
                .find(|(name, _)| *name == family);
            match width {
                Some((_, precision)) => Some(format!("Decimal({precision}, {})", arguments.trim())),
                None => Some(format!("{family}({})", listed(arguments))),
            }
        }
    }
}
