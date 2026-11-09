//! Text pieces of the index rendering.

use crate::index::models::{Dependents, KeyedEntry};

/// What a section header says about one identifier.
pub(crate) struct Header<'a> {
    pub(crate) section: &'a str,
    pub(crate) id: &'a str,
    pub(crate) rules: Option<usize>,
}

/// A blank line, then `<section> <id>: [<n> rules, ]<n> cases, <n> files`.
pub(crate) fn header(header: &Header<'_>, of: &Dependents) -> String {
    let rules = header
        .rules
        .map_or_else(String::new, |count| format!("{count} rules, "));
    format!(
        "\n{} {}: {rules}{} cases, {} files\n",
        header.section,
        header.id,
        of.cases.len(),
        of.files.len()
    )
}

/// One `  <label> <item>` line per item.
pub(crate) fn lines(label: &str, items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("  {label} {item}\n"))
        .collect()
}

/// The case lines, then the file lines.
pub(crate) fn dependents(of: &Dependents) -> String {
    lines("case", &of.cases) + &lines("file", &of.files)
}

/// Headers and dependents of equivalences or refused constructs.
pub(crate) fn keyed(section: &str, entries: &[KeyedEntry]) -> String {
    let mut text = String::new();
    for entry in entries {
        let title = Header {
            section,
            id: &entry.id,
            rules: None,
        };
        text.push_str(&header(&title, &entry.dependents));
        text.push_str(&dependents(&entry.dependents));
    }
    text
}
