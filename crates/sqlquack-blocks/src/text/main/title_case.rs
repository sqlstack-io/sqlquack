//! `title_case`: INITCAP, each word's first character upper-cased and the rest lower-cased.

use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::constants::UPPERCASE_EXPANSIONS;
use crate::blocks::main::full_case_lower::full_case_lower;
use crate::blocks::main::string_literal::string_literal;
use crate::raises::main::raised::raised;
use crate::text::constants::{INITCAP_TEXT, TITLE_CASE, TITLE_CASE_DIGRAPHS, TITLE_CASE_KNOWN};

/// Each word capitalised after `delimiters`; characters no recording shows raise `initcap_text`.
pub fn title_case(value: &str, delimiters: &str, log: &ProvenanceLog) -> String {
    log.building_block(TITLE_CASE);
    let text = "x";
    let initials: String = UPPERCASE_EXPANSIONS.iter().map(|(from, _)| *from).collect();
    let delimiter_class: String = delimiters
        .chars()
        .map(|character| match character {
            '-' | ']' | '\\' | '^' | '[' => format!("\\{character}"),
            _ => character.to_string(),
        })
        .collect();
    let unknown = match delimiter_class.is_empty() {
        true => format!("[^{TITLE_CASE_KNOWN}]|[{TITLE_CASE_DIGRAPHS}]|^[{initials}]"),
        false => format!(
            "[^{TITLE_CASE_KNOWN}]|[{TITLE_CASE_DIGRAPHS}]|(^|[{delimiter_class}])[{initials}]"
        ),
    };
    let unproven = raised(&format!(
        "{INITCAP_TEXT}: text with a character no recording shows INITCAP reading"
    ));
    let after_delimiter = match delimiters.is_empty() {
        true => String::new(),
        false => format!(
            " OR list_contains({}, substring({text}, i - 1, 1))",
            delimiter_list(delimiters, log)
        ),
    };
    let body = format!(
        "(CASE WHEN {text} IS NULL THEN NULL WHEN regexp_matches({text}, {}) \
         THEN CAST({unproven} AS VARCHAR) \
         ELSE array_to_string(list_transform(string_split({text}, ''), lambda c, i: \
         CASE WHEN i = 1{after_delimiter} THEN upper(c) ELSE {} END), '') END)",
        string_literal(&unknown, log),
        full_case_lower("c", log)
    );
    format!("list_transform([{value}], lambda {text}: {body})[1]")
}

/// The delimiters as a list of one-character strings.
fn delimiter_list(delimiters: &str, log: &ProvenanceLog) -> String {
    let items: Vec<String> = delimiters
        .chars()
        .map(|character| string_literal(&character.to_string(), log))
        .collect();
    format!("[{}]", items.join(", "))
}
