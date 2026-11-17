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

/// The byte ranges of `sql`'s space-separated tokens.
pub(crate) fn space_tokens(sql: &str) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut start: usize = 0;
    for (index, character) in sql.char_indices().chain(std::iter::once((sql.len(), ' '))) {
        if character == ' ' {
            if index > start {
                out.push((start, index));
            }
            start = index + 1;
        }
    }
    out
}

/// `sql` without the token at `start..end`, removing one space beside it, or keeping its neighbours apart.
fn dropped(sql: &str, (start, end): (usize, usize)) -> String {
    let before: Option<char> = sql[..start].chars().next_back();
    let after: Option<char> = sql[end..].chars().next();
    let (from, to, gap) = match (before, after) {
        (_, Some(' ')) => (start, end + 1, ""),
        (Some(' '), None) => (start - 1, end, ""),
        (Some(_), Some(_)) if before != Some(' ') => (start, end, " "),
        _ => (start, end, ""),
    };
    format!("{}{gap}{}", &sql[..from], &sql[to..])
}

/// Every distinct, non-blank text dropping or respelling one space-separated or `lexed` token of `sql`.
pub(crate) fn mutations(sql: &str, lexed: &[(usize, usize)]) -> Vec<Mutation> {
    let mut tokens: Vec<(usize, usize)> = space_tokens(sql);
    tokens.extend(lexed.iter().copied().filter(|(start, end)| start < end));
    tokens.sort_unstable();
    tokens.dedup();
    let mut seen: Vec<String> = vec![sql.to_owned()];
    let mut out: Vec<Mutation> = Vec::new();
    for (position, &(start, end)) in tokens.iter().enumerate() {
        let token: &str = &sql[start..end];
        let changes = std::iter::once((MutationKind::Drop, dropped(sql, (start, end)))).chain(
            respellings(token)
                .into_iter()
                .map(|(kind, text)| (kind, format!("{}{text}{}", &sql[..start], &sql[end..]))),
        );
        for (kind, text) in changes {
            if text.trim().is_empty() || seen.contains(&text) {
                continue;
            }
            seen.push(text.clone());
            out.push(Mutation {
                kind,
                token: token.to_owned(),
                position,
                last: sql[end..].trim().is_empty(),
                sql: text,
            });
        }
    }
    out
}
