//! `is_unquoted_word`: whether a token is a word whose letter case carries no meaning.

/// Whether `text` is an unquoted ASCII word whose letter case carries no meaning.
pub(crate) fn is_unquoted_word(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '$'))
}
