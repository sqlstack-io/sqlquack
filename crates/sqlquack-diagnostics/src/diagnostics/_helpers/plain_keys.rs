//! Construct keys in a message, such as `fn:HASH`, written as SQL readers name them.

use crate::classification::models::{Construct, ConstructFamily};

/// The families whose `<family>:<name>` keys messages may carry.
const FAMILIES: [ConstructFamily; 6] = [
    ConstructFamily::Node,
    ConstructFamily::Function,
    ConstructFamily::Cast,
    ConstructFamily::Join,
    ConstructFamily::Clause,
    ConstructFamily::Syntax,
];

/// `text` with every construct key replaced by the construct's plain name.
pub(crate) fn plain_keys(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((start, family, name_start)) = next_key(rest) {
        let name_length = rest[name_start..]
            .find(|character: char| !is_name_character(character))
            .unwrap_or(rest.len() - name_start);
        plain.push_str(&rest[..start]);
        let name = &rest[name_start..name_start + name_length];
        plain.push_str(&Construct::new(family, name).plain_name());
        rest = &rest[name_start + name_length..];
    }
    plain.push_str(rest);
    plain
}

/// The first key in `text`: where it starts, its family, and where its name starts.
fn next_key(text: &str) -> Option<(usize, ConstructFamily, usize)> {
    let mut first: Option<(usize, ConstructFamily, usize)> = None;
    for family in FAMILIES {
        let prefix = format!("{}:", family.prefix());
        let found = text
            .match_indices(&prefix)
            .map(|(start, _)| start)
            .find(|start| is_key_at(text, *start, prefix.len()));
        if let Some(start) = found
            && first.is_none_or(|(earliest, _, _)| start < earliest)
        {
            first = Some((start, family, start + prefix.len()));
        }
    }
    first
}

/// Whether a key prefix `prefix_length` long at `start` begins a word and is followed by a name.
fn is_key_at(text: &str, start: usize, prefix_length: usize) -> bool {
    let boundary = text[..start]
        .chars()
        .next_back()
        .is_none_or(|before| !before.is_ascii_alphanumeric() && before != '_');
    let named = text[start + prefix_length..]
        .chars()
        .next()
        .is_some_and(is_name_character);
    boundary && named
}

fn is_name_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '<' | '>')
}
