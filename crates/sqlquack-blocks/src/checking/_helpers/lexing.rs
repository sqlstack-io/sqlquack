//! The statements and names of printed SQL, read lexically without DuckDB's serialiser.

use std::collections::BTreeSet;

/// What a lexical pass over printed SQL finds.
pub(crate) struct Lexed {
    /// Statements separated by semicolons outside strings, quoted names and comments.
    pub(crate) statements: usize,
    /// Every quoted identifier, bare word and dotted chain of them, any of which may name a relation.
    pub(crate) names: BTreeSet<String>,
}

/// Statements and candidate names of `sql`; strings, quoted names and comments are skipped.
pub(crate) fn lexed(sql: &str) -> Lexed {
    let characters: Vec<char> = sql.chars().collect();
    let mut names: BTreeSet<String> = BTreeSet::new();
    let (mut statements, mut pending) = (0_usize, false);
    let (mut chain, mut continuing) = (String::new(), false);
    let mut index = 0;
    while let Some(&character) = characters.get(index) {
        let next = characters.get(index + 1).copied();
        match (character, next) {
            ('\'', _) => {
                chain.clear();
                index = quoted_end(&characters, index, '\'').0;
                pending = true;
            }
            ('"', _) => {
                let (end, name) = quoted_end(&characters, index, '"');
                chain = extended(&chain, std::mem::take(&mut continuing), &name);
                names.extend([chain.clone(), name]);
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
                let name: String = characters[start..index].iter().collect();
                chain = extended(&chain, std::mem::take(&mut continuing), &name);
                names.extend([chain.clone(), name]);
                pending = true;
            }
            ('.', _) if !chain.is_empty() && !continuing => {
                continuing = true;
                pending = true;
                index += 1;
            }
            (character, _) => {
                pending |= !character.is_whitespace();
                chain.clear();
                continuing = false;
                index += 1;
            }
        }
    }
    Lexed {
        statements: statements + usize::from(pending),
        names,
    }
}

/// The dotted chain `name` ends: `chain.name` when a dot joined it to `chain`, else `name` alone.
fn extended(chain: &str, continuing: bool, name: &str) -> String {
    match continuing && !chain.is_empty() {
        true => format!("{chain}.{name}"),
        false => name.to_owned(),
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
