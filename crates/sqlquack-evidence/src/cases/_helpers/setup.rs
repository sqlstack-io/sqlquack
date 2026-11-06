//! The setup a case runs before its SQL: the statements it may hold and the placeholders it names.

use crate::cases::models::CaseDefinition;
use crate::constants::{SETUP_PLACEHOLDERS, SETUP_STATEMENT_STARTS};

/// Problems with a case's setup statements and with the placeholders its setup and SQL name.
pub(crate) fn setup_problems(label: &str, case: &CaseDefinition) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    if !case.setup.is_empty() && !case.functions.is_empty() {
        problems.push(format!(
            "{label}: a case with `setup` cannot also define functions"
        ));
    }
    for statement in &case.setup {
        let words: String = statement
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
            .to_ascii_uppercase();
        if !SETUP_STATEMENT_STARTS
            .iter()
            .any(|start| format!("{words} ").starts_with(start))
        {
            problems.push(format!(
                "{label}: setup statement `{statement}` is not CREATE TABLE, VIEW, TYPE or SEMANTIC VIEW, INSERT INTO, SET, UNSET or ALTER SESSION"
            ));
        }
        if statement.contains(';') {
            problems.push(format!(
                "{label}: setup statement `{statement}` must be one statement, without `;`"
            ));
        }
    }
    let texts = case
        .setup
        .iter()
        .chain(&case.expression)
        .chain(&case.query)
        .chain(&case.canonical);
    for text in texts {
        for placeholder in placeholders(text) {
            if !SETUP_PLACEHOLDERS.contains(&placeholder) {
                problems.push(format!("{label}: unknown placeholder `{placeholder}`"));
            } else if case.setup.is_empty() {
                problems.push(format!(
                    "{label}: placeholder `{placeholder}` needs a case with `setup`"
                ));
            }
        }
    }
    problems
}

/// Every `{{...}}` in `text`, braces included, in order.
fn placeholders(text: &str) -> Vec<&str> {
    let mut found: Vec<&str> = Vec::new();
    let mut rest: &str = text;
    while let Some(start) = rest.find("{{") {
        let tail: &str = &rest[start..];
        let end: usize = tail.find("}}").map_or(tail.len(), |end| end + 2);
        found.push(&tail[..end]);
        rest = &tail[end..];
    }
    found
}
