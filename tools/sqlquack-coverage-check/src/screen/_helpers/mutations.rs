//! `mutations`: the one-token variants of a case's text.

use crate::screen::constants::SYNONYMS;
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

/// The text inside `token` when it is a string in `quote` with no quote or escape inside.
fn simple_string(token: &str, quote: char) -> Option<&str> {
    token
        .strip_prefix(quote)
        .and_then(|rest| rest.strip_suffix(quote))
        .filter(|text| !text.contains(['\'', '"', '\\']))
}

/// The other spellings of one token, each with how it changes the token.
fn respellings(token: &str) -> Vec<(MutationKind, String)> {
    let spells = |group: &&&[&str]| group.iter().any(|word| token.eq_ignore_ascii_case(word));
    let mut out: Vec<(MutationKind, String)> = SYNONYMS
        .iter()
        .filter(spells)
        .flat_map(|group| group.iter())
        .filter(|word| !token.eq_ignore_ascii_case(word))
        .map(|word| (MutationKind::Swap, (*word).to_owned()))
        .collect();
    if is_name(token) {
        out.push((MutationKind::Backquote, format!("`{token}`")));
        out.push((MutationKind::String, format!("'{token}'")));
        out.push((MutationKind::DoubleQuote, format!("\"{token}\"")));
    } else if let Some(name) = enclosed(token, '`') {
        out.push((MutationKind::Unquote, name.to_owned()));
        out.push((MutationKind::String, format!("'{name}'")));
        out.push((MutationKind::DoubleQuote, format!("\"{name}\"")));
    } else if let Some(name) = enclosed(token, '\'') {
        out.push((MutationKind::Unquote, name.to_owned()));
        out.push((MutationKind::Backquote, format!("`{name}`")));
    } else if let Some(name) = enclosed(token, '"') {
        out.push((MutationKind::Unquote, name.to_owned()));
        out.push((MutationKind::Backquote, format!("`{name}`")));
    }
    if let Some(text) = simple_string(token, '\'') {
        out.push((MutationKind::Requote, format!("\"{text}\"")));
    } else if let Some(text) = simple_string(token, '"') {
        out.push((MutationKind::Requote, format!("'{text}'")));
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
        (None | Some(' '), Some(' ')) => (start, end + 1, ""),
        (Some(' '), None) => (start - 1, end, ""),
        (Some(_), Some(after)) if before != Some(' ') && after != ' ' => (start, end, " "),
        _ => (start, end, ""),
    };
    format!("{}{gap}{}", &sql[..from], &sql[to..])
}

/// `sql` with `word` written at byte `at`, kept apart from the text on both sides.
fn inserted(sql: &str, at: usize, word: &str) -> String {
    let (head, tail) = sql.split_at(at);
    let left: &str = match head.chars().next_back() {
        None | Some(' ') => "",
        Some(_) => " ",
    };
    let right: &str = match tail.chars().next() {
        None | Some(' ') => "",
        Some(_) => " ",
    };
    format!("{head}{left}{word}{right}{tail}")
}

/// Every distinct, non-blank text dropping, respelling or preceding with `AS` or `=` one token.
pub(crate) fn mutations(sql: &str, lexed: &[(usize, usize)]) -> Vec<Mutation> {
    let lexed: Vec<(usize, usize)> = lexed
        .iter()
        .copied()
        .filter(|(start, end)| start < end)
        .collect();
    let mut tokens: Vec<(usize, usize)> = space_tokens(sql);
    tokens.extend(lexed.iter().copied());
    tokens.sort_unstable();
    tokens.dedup();
    let covered = |(start, end): (usize, usize)| lexed.iter().any(|&(s, e)| s < end && start < e);
    let mut seen: Vec<String> = vec![sql.to_owned()];
    let mut out: Vec<Mutation> = Vec::new();
    let mut push = |kind: MutationKind,
                    token: &str,
                    (position, last, untokenized): (usize, bool, bool),
                    text: String| {
        if text.trim().is_empty() || seen.contains(&text) {
            return;
        }
        seen.push(text.clone());
        out.push(Mutation {
            kind,
            token: token.to_owned(),
            position,
            last,
            untokenized,
            sql: text,
        });
    };
    for (position, &(start, end)) in tokens.iter().enumerate() {
        let token: &str = &sql[start..end];
        let place = (
            position,
            sql[end..].trim().is_empty(),
            !covered((start, end)),
        );
        push(MutationKind::Drop, token, place, dropped(sql, (start, end)));
        for (kind, text) in respellings(token) {
            push(
                kind,
                token,
                place,
                format!("{}{text}{}", &sql[..start], &sql[end..]),
            );
        }
        push(
            MutationKind::InsertAs,
            token,
            place,
            inserted(sql, start, "AS"),
        );
        push(
            MutationKind::InsertEquals,
            token,
            place,
            inserted(sql, start, "="),
        );
    }
    let end = (tokens.len(), true, false);
    push(
        MutationKind::InsertAs,
        "",
        end,
        inserted(sql, sql.len(), "AS"),
    );
    push(
        MutationKind::InsertEquals,
        "",
        end,
        inserted(sql, sql.len(), "="),
    );
    out
}
