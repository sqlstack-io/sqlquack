//! `canonical_type`: a type spelling in the form the default type comparison uses.

/// A type spelling without spaces, upper case, with the common synonyms of INT and text folded.
pub fn canonical_type(spelling: &str) -> String {
    let compact: String = spelling
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    match compact.as_str() {
        "INTEGER" => "INT".to_owned(),
        "VARCHAR" | "STRING" => "TEXT".to_owned(),
        _ => compact,
    }
}
