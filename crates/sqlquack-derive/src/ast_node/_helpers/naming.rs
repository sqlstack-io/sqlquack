//! Naming.

/// Match serde's `rename_all = "snake_case"` behavior for Rust enum variants.
pub(crate) fn serde_snake_case(name: &str) -> String {
    let mut snake_case = String::with_capacity(name.len());
    for (index, character) in name.chars().enumerate() {
        if index > 0 && character.is_uppercase() {
            snake_case.push('_');
        }
        snake_case.extend(character.to_lowercase());
    }
    snake_case
}
