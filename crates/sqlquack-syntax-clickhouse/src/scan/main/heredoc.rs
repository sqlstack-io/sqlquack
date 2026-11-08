//! The body of a heredoc token, as ClickHouse's `ParserStringLiteral` reads it.

/// The body of a `$tag$...$tag$` token.
pub fn heredoc_body(text: &str) -> String {
    let Some(tag_end) = text[1..].find('$') else {
        return text.to_owned();
    };
    let tag_len: usize = tag_end + 2;
    match text.len() >= 2 * tag_len {
        true => text[tag_len..text.len() - tag_len].to_owned(),
        false => text.to_owned(),
    }
}
