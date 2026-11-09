//! SQL text with its comments blanked out.

/// The text with each comment, outside quotes, replaced by a space.
pub(crate) fn uncommented(sql: &str) -> String {
    let characters: Vec<char> = sql.chars().collect();
    let mut text = String::with_capacity(sql.len());
    let mut index = 0;
    while let Some(&character) = characters.get(index) {
        let next = characters.get(index + 1).copied();
        let end = match (character, next) {
            ('-', Some('-')) | ('/', Some('/')) => line_end(&characters, index),
            ('/', Some('*')) => block_end(&characters, index + 2),
            ('\'' | '"', _) => quote_end(&characters, index + 1, character),
            _ => index + 1,
        };
        match (character, next) {
            ('-', Some('-')) | ('/', Some('/' | '*')) => text.push(' '),
            _ => text.extend(&characters[index..end]),
        }
        index = end;
    }
    text
}

/// The index after the line feed ending a line comment at `start`, or the end of the text.
fn line_end(characters: &[char], start: usize) -> usize {
    characters[start..]
        .iter()
        .position(|character| *character == '\n')
        .map_or(characters.len(), |offset| start + offset + 1)
}

/// The index after the `*/` closing a block comment whose body starts at `start`.
fn block_end(characters: &[char], start: usize) -> usize {
    characters
        .get(start..)
        .unwrap_or_default()
        .windows(2)
        .position(|pair| pair == ['*', '/'])
        .map_or(characters.len(), |offset| start + offset + 2)
}

/// The index after the quote closing quoted text whose body starts at `start`.
fn quote_end(characters: &[char], start: usize, quote: char) -> usize {
    let mut index = start;
    while let Some(&character) = characters.get(index) {
        index += match character {
            '\\' if quote == '\'' => 2,
            _ if character == quote => return index + 1,
            _ => 1,
        };
    }
    characters.len()
}
