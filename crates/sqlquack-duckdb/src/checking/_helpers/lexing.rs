//! The statements and names of printed SQL, read lexically without DuckDB's serialiser.

use std::collections::BTreeSet;

/// What a lexical pass over printed SQL finds.
pub(crate) struct Lexed {
    /// Statements separated by semicolons outside strings, quoted names and comments.
    pub(crate) statements: usize,
    /// Every quoted identifier and bare word, any of which may name a relation or function.
    pub(crate) names: BTreeSet<String>,
}

/// Statements and candidate names of `sql`; strings, quoted names and comments are skipped.
pub(crate) fn lexed(sql: &str) -> Lexed {
    let characters: Vec<char> = sql.chars().collect();
    let mut names: BTreeSet<String> = BTreeSet::new();
    let (mut statements, mut pending) = (0_usize, false);
    let mut index = 0;
    while let Some(&character) = characters.get(index) {
        let next = characters.get(index + 1).copied();
        match (character, next) {
            ('\'', _) => {
                index = quoted_end(&characters, index, '\'').0;
                pending = true;
            }
            ('"', _) => {
                let (end, name) = quoted_end(&characters, index, '"');
                names.insert(name);
                index = end;
                pending = true;
            }
            ('-', Some('-')) => {
                while characters
                    .get(index)
                    .is_some_and(|character| *character != '\n')
                {
                    index += 1;
                }
            }
            ('/', Some('*')) => {
                index += 2;
                while index < characters.len()
                    && !(characters[index] == '*' && characters.get(index + 1) == Some(&'/'))
                {
                    index += 1;
                }
                index += 2;
            }
            (';', _) => {
                statements += usize::from(pending);
                pending = false;
                index += 1;
            }
            (character, _) if character.is_alphanumeric() || character == '_' => {
                let start = index;
                while characters
                    .get(index)
                    .is_some_and(|character| character.is_alphanumeric() || *character == '_')
                {
                    index += 1;
                }
                names.insert(characters[start..index].iter().collect());
                pending = true;
            }
            (character, _) => {
                pending |= !character.is_whitespace();
                index += 1;
            }
        }
    }
    Lexed {
        statements: statements + usize::from(pending),
        names,
    }
}

/// The index past the quoted text opening at `start`, and its text with doubled quotes undone.
fn quoted_end(characters: &[char], start: usize, quote: char) -> (usize, String) {
    let mut text = String::new();
    let mut index = start + 1;
    while let Some(&character) = characters.get(index) {
        match (
            character == quote,
            characters.get(index + 1) == Some(&quote),
        ) {
            (true, true) => {
                text.push(quote);
                index += 2;
            }
            (true, false) => return (index + 1, text),
            (false, _) => {
                text.push(character);
                index += 1;
            }
        }
    }
    (index, text)
}
