//! `mutations`: the one-token variants of a case's text.

use crate::screen::constants::SWAPPED_KEYWORDS;
use crate::screen::models::{Mutation, MutationKind};

/// A name a bare identifier could spell: a letter or underscore, then letters, digits or underscores.
fn is_name(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// The name inside `token` when it is enclosed by `quote` on both sides.
fn enclosed(token: &str, quote: char) -> Option<&str> {
    token
        .strip_prefix(quote)
        .and_then(|rest| rest.strip_suffix(quote))
        .filter(|name| is_name(name))
}

/// The other spellings of one token, each with how it changes the token.
fn respellings(token: &str) -> Vec<(MutationKind, String)> {
    let mut out: Vec<(MutationKind, String)> = SWAPPED_KEYWORDS
        .iter()
        .filter(|(from, _)| token.eq_ignore_ascii_case(from))
        .map(|(_, to)| (MutationKind::Swap, (*to).to_owned()))
        .collect();
    if is_name(token) {
        out.push((MutationKind::Backquote, format!("`{token}`")));
        out.push((MutationKind::String, format!("'{token}'")));
    } else if let Some(name) = enclosed(token, '`') {
        out.push((MutationKind::Unquote, name.to_owned()));
        out.push((MutationKind::String, format!("'{name}'")));
    } else if let Some(name) = enclosed(token, '\'') {
        out.push((MutationKind::Unquote, name.to_owned()));
        out.push((MutationKind::Backquote, format!("`{name}`")));
    }
    out
}

/// Every distinct, non-blank text that drops or respells one space-separated token of `sql`.
pub(crate) fn mutations(sql: &str) -> Vec<Mutation> {
    let tokens: Vec<&str> = sql.split(' ').collect();
    let mut seen: Vec<String> = vec![sql.to_owned()];
    let mut out: Vec<Mutation> = Vec::new();
    for (position, token) in tokens.iter().enumerate() {
        let changes = std::iter::once((MutationKind::Drop, None)).chain(
            respellings(token)
                .into_iter()
                .map(|(kind, text)| (kind, Some(text))),
        );
        for (kind, replacement) in changes {
            let mut written: Vec<&str> = tokens.clone();
            match &replacement {
                Some(text) => written[position] = text,
                None => {
                    written.remove(position);
                }
            }
            let text: String = written.join(" ");
            if text.trim().is_empty() || seen.contains(&text) {
                continue;
            }
            seen.push(text.clone());
            out.push(Mutation {
                kind,
                token: (*token).to_owned(),
                position,
                sql: text,
            });
        }
    }
    out
}
