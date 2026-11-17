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

/// The byte ranges inside the delimiters of each comment in the text the `lexed` tokens leave.
pub(crate) fn comment_bodies(sql: &str, lexed: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut gaps: Vec<(usize, usize)> = Vec::new();
    let mut from: usize = 0;
    for &(start, end) in lexed {
        if start > from {
            gaps.push((from, start));
        }
        from = from.max(end);
    }
    gaps.push((from, sql.len()));
    let mut out: Vec<(usize, usize)> = Vec::new();
    for (start, end) in gaps {
        let gap: &str = &sql[start..end];
        let mut at: usize = 0;
        while at < gap.len() {
            let rest: &str = &gap[at..];
            let body: Option<(usize, usize)> = match () {
                () if rest.starts_with("/*") => nested_close(rest).map(|close| (2, close)),
                () if rest.starts_with("--") => Some((2, rest.find('\n').unwrap_or(rest.len()))),
                () => None,
            };
            match body {
                Some((open, close)) if close >= open => {
                    out.push((start + at + open, start + at + close));
                    at += close.max(open);
                }
                _ => at += rest.chars().next().map_or(1, char::len_utf8),
            }
        }
    }
    out
}

/// Where the `*/` closing the `/*` that opens `text` starts, counting nested comments as Spark does.
fn nested_close(text: &str) -> Option<usize> {
    let mut depth: usize = 0;
    let mut at: usize = 0;
    while at + 1 < text.len() {
        match &text.as_bytes()[at..at + 2] {
            b"/*" => {
                depth += 1;
                at += 2;
            }
            b"*/" => {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
                at += 2;
            }
            _ => at += 1,
        }
    }
    None
}

/// The byte ranges of each `(` among the `lexed` tokens and its matching `)`.
fn paren_pairs(sql: &str, lexed: &[(usize, usize)]) -> Vec<((usize, usize), (usize, usize))> {
    let mut open: Vec<(usize, usize)> = Vec::new();
    let mut out: Vec<((usize, usize), (usize, usize))> = Vec::new();
    for &(start, end) in lexed {
        match &sql[start..end] {
            "(" => open.push((start, end)),
            ")" => out.extend(open.pop().map(|left| (left, (start, end)))),
            _ => {}
        }
    }
    out
}

/// Every distinct, non-blank one-token or parenthesis-pair change to `sql` that could collapse.
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
    let bodies: Vec<(usize, usize)> = comment_bodies(sql, &lexed);
    let inside =
        |(start, end): (usize, usize)| bodies.iter().any(|&(from, to)| from <= start && end <= to);
    let mut seen: Vec<String> = vec![sql.to_owned()];
    let mut out: Vec<Mutation> = Vec::new();
    let mut push = |kind: MutationKind,
                    token: &str,
                    (position, last, changed): (usize, bool, (usize, usize)),
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
            untokenized: inside(changed),
            sql: text,
        });
    };
    for (position, &(start, end)) in tokens.iter().enumerate() {
        let token: &str = &sql[start..end];
        let last: bool = sql[end..].trim().is_empty();
        let whole = (position, last, (start, end));
        let before = (position, last, (start, start));
        push(MutationKind::Drop, token, whole, dropped(sql, (start, end)));
        for (kind, text) in respellings(token) {
            push(
                kind,
                token,
                whole,
                format!("{}{text}{}", &sql[..start], &sql[end..]),
            );
        }
        push(
            MutationKind::InsertAs,
            token,
            before,
            inserted(sql, start, "AS"),
        );
        push(
            MutationKind::InsertEquals,
            token,
            before,
            inserted(sql, start, "="),
        );
        if lexed.contains(&(start, end)) && !matches!(token, "(" | ")") {
            let wrapped: String = format!("{}( {token} ){}", &sql[..start], &sql[end..]);
            push(MutationKind::ParenPair, token, whole, wrapped);
        }
    }
    for (open, close) in paren_pairs(sql, &lexed) {
        let position: usize = tokens.iter().position(|token| *token == open).unwrap_or(0);
        let text: String = dropped(&dropped(sql, close), open);
        let place = (
            position,
            sql[close.1..].trim().is_empty(),
            (open.0, close.1),
        );
        push(MutationKind::ParenPair, "(", place, text);
    }
    let end = (tokens.len(), true, (sql.len(), sql.len()));
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
