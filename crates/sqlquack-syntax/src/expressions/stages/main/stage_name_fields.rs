//! `stage_name_fields`: split a written stage location into its name parts and the text after them.

use crate::expressions::names::models::Identifier;
use crate::expressions::stages::models::StageName;

/// A stage location's written name split into its parts, and the text after the name, such as a `/` path.
pub fn stage_name_fields(text: &str) -> (StageName, &str) {
    let body = text.strip_prefix('@').unwrap_or(text);
    if let Some(rest) = body.strip_prefix('~') {
        let name = StageName {
            parts: Vec::new(),
            table_stage: false,
            user_stage: true,
        };
        return (name, rest);
    }
    let mut parts: Vec<Option<Identifier>> = Vec::new();
    let mut table_stage = false;
    let mut at = 0;
    loop {
        if body[at..].starts_with('%') {
            table_stage = true;
            at += 1;
        }
        if body[at..].starts_with('"') {
            let mut name = String::new();
            let mut chars = body[at + 1..].char_indices().peekable();
            let mut end = body.len();
            while let Some((offset, character)) = chars.next() {
                if character != '"' {
                    name.push(character);
                    continue;
                }
                if chars.peek().is_some_and(|(_, next)| *next == '"') {
                    chars.next();
                    name.push('"');
                    continue;
                }
                end = at + 1 + offset + 1;
                break;
            }
            at = end;
            parts.push(Some(Identifier::quoted(name)));
        } else {
            let end = body[at..]
                .find(['.', '/'])
                .map_or(body.len(), |offset| at + offset);
            let name = &body[at..end];
            parts.push((!name.is_empty()).then(|| Identifier::new(name)));
            at = end;
        }
        match body[at..].strip_prefix('.') {
            Some(_) => at += 1,
            None => break,
        }
    }
    let name = StageName {
        parts,
        table_stage,
        user_stage: false,
    };
    (name, &body[at..])
}
