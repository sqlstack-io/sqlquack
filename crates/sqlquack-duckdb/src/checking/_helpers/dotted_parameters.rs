//! Lambda parameters read with a dot, which a relation of the same alias captures in DuckDB.

use std::collections::BTreeSet;

use crate::checking::constants::LAMBDA_KEYWORD;

/// Lambda parameters of `sql` read with a dot, outside string literals and quoted identifiers.
pub(crate) fn dotted_parameters(sql: &str) -> Vec<String> {
    let bare = without_quoted(sql);
    let parameters: BTreeSet<String> = bare
        .match_indices(LAMBDA_KEYWORD)
        .flat_map(|(start, _)| lambda_parameters(&bare[start + LAMBDA_KEYWORD.len()..]))
        .collect();
    parameters
        .into_iter()
        .filter(|name| is_dotted(&bare, name))
        .collect()
}

/// The comma-separated names before a lambda's colon.
fn lambda_parameters(after_keyword: &str) -> Vec<String> {
    let head = after_keyword.split(':').next().unwrap_or_default();
    head.split(',').map(|name| name.trim().to_owned()).collect()
}

/// Whether `name` stands alone before a dot and a field name somewhere in `bare`.
fn is_dotted(bare: &str, name: &str) -> bool {
    let word = |character: char| character.is_alphanumeric() || character == '_';
    bare.match_indices(&format!("{name}.")).any(|(start, _)| {
        let before = bare[..start].chars().next_back();
        let after = bare[start + name.len() + 1..].chars().next();
        !before.is_some_and(word)
            && after.is_some_and(|character| character.is_alphabetic() || character == '_')
    })
}

/// `sql` with the text of string literals and quoted identifiers blanked.
fn without_quoted(sql: &str) -> String {
    let mut quote: Option<char> = None;
    sql.chars()
        .map(|character| match (quote, character) {
            (None, '\'' | '"') => {
                quote = Some(character);
                ' '
            }
            (Some(open), _) if character == open => {
                quote = None;
                ' '
            }
            (Some(_), _) => ' ',
            (None, _) => character,
        })
        .collect()
}
