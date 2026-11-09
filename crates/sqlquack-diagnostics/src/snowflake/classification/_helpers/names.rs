//! Type name spellings for construct keys.

pub(in crate::snowflake) fn base_type_name(rendered: &str) -> String {
    let base = rendered
        .split(['(', '<', ' '])
        .next()
        .unwrap_or(rendered)
        .trim();
    base.to_ascii_uppercase()
}
