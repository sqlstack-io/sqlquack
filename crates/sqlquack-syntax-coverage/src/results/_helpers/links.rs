//! Which catalogue forms a recorded statement exercises, from the constructs it uses.

use std::collections::{BTreeMap, BTreeSet};

use crate::formats::main::content_sha256::content_sha256;
use crate::formats::models::{FormStatus, SyntaxCoverage};
use crate::results::constants::{
    FUNCTION_PREFIX, FUNCTIONS_FAMILY, NAMED_ARGUMENT, STATEMENT_REFUSAL, WINDOW_FUNCTION,
};
use crate::results::models::{LinksFile, RecordedStatement};

/// A function form a call links to, and the construct its syntax needs beside the call, if any.
pub(crate) type FunctionForm = (String, Option<&'static str>);

/// Function forms by the upper-case name their grammar calls, and the SHA-256 of that index.
pub(crate) struct FunctionIndex {
    pub(crate) forms: BTreeMap<String, Vec<FunctionForm>>,
    pub(crate) sha256: String,
}

/// The forms a statement links to, and those its refusal names.
pub(crate) struct StatementLinks {
    pub(crate) forms: BTreeSet<String>,
    /// The forms of its refused constructs; every linked form when they name none or the whole.
    pub(crate) refused_forms: BTreeSet<String>,
    /// Constructs that neither link nor name no form.
    pub(crate) unaccounted: Vec<String>,
}

/// The name a call grammar starts with, such as `SNOWFLAKE.CORTEX.COMPLETE` in `...COMPLETE(`.
fn call_name(grammar: &str) -> Option<String> {
    let grammar = grammar.trim_start();
    let end = grammar
        .find(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '_' | '$' | '.'))
        })
        .unwrap_or(grammar.len());
    let (name, rest) = grammar.split_at(end);
    let starts = name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_');
    (starts && rest.trim_start().starts_with('(')).then(|| name.to_ascii_uppercase())
}

/// The construct a function form's own syntax needs: OVER a window, or named arguments.
fn needed(grammar: &str) -> Option<&'static str> {
    let upper = grammar.to_ascii_uppercase();
    if upper.contains(" OVER ") || upper.contains(" OVER(") {
        return Some(WINDOW_FUNCTION);
    }
    upper.contains("=>").then_some(NAMED_ARGUMENT)
}

/// One form in the hashed index text: its id, and `+` the construct its syntax needs.
fn index_entry((id, needed): &FunctionForm) -> String {
    match needed {
        Some(key) => format!("{id}+{key}"),
        None => id.clone(),
    }
}

/// Every catalogued function form under the name its first signature calls.
pub(crate) fn function_index(coverage: &SyntaxCoverage) -> FunctionIndex {
    let mut forms: BTreeMap<String, Vec<FunctionForm>> = BTreeMap::new();
    for form in coverage
        .catalogue
        .get(FUNCTIONS_FAMILY)
        .into_iter()
        .flatten()
    {
        if form.status == FormStatus::AbsentFromSource {
            continue;
        }
        let Some(grammar) = form.grammar.first() else {
            continue;
        };
        if let Some(name) = call_name(grammar) {
            forms
                .entry(name)
                .or_default()
                .push((form.id.clone(), needed(grammar)));
        }
    }
    let mut text: String = String::new();
    for (name, ids) in &forms {
        let ids: Vec<String> = ids.iter().map(index_entry).collect();
        text.push_str(&format!("{name} {}\n", ids.join(",")));
    }
    FunctionIndex {
        sha256: content_sha256(text.as_bytes()),
        forms,
    }
}

/// The forms `construct` links to in a statement using `used`, and whether anything accounts for it.
fn construct_forms(
    construct: &str,
    used: &BTreeSet<&str>,
    functions: &FunctionIndex,
    links: &LinksFile,
) -> (BTreeSet<String>, bool) {
    let mut forms: BTreeSet<String> = BTreeSet::new();
    let mut accounted = false;
    let named = construct
        .strip_prefix(FUNCTION_PREFIX)
        .and_then(|name| functions.forms.get(name));
    for (id, needed) in named.into_iter().flatten() {
        accounted = true;
        if needed.is_none_or(|key| used.contains(key)) {
            forms.insert(id.clone());
        }
    }
    for link in &links.links {
        if !link.constructs.iter().any(|key| key == construct) {
            continue;
        }
        accounted = true;
        if link
            .constructs
            .iter()
            .all(|key| used.contains(key.as_str()))
        {
            forms.extend(link.forms.iter().cloned());
        }
    }
    for no_form in &links.no_forms {
        accounted |= no_form.constructs.iter().any(|key| key == construct);
    }
    (forms, accounted)
}

/// The forms `statement` links to, those its refusal names, and its unaccounted constructs.
pub(crate) fn link_statement(
    statement: &RecordedStatement,
    functions: &FunctionIndex,
    links: &LinksFile,
) -> StatementLinks {
    let used: BTreeSet<&str> = statement.constructs.iter().map(String::as_str).collect();
    let mut forms: BTreeSet<String> = BTreeSet::new();
    let mut unaccounted: Vec<String> = Vec::new();
    for construct in &statement.constructs {
        let (linked, accounted) = construct_forms(construct, &used, functions, links);
        forms.extend(linked);
        if !accounted {
            unaccounted.push(construct.clone());
        }
    }
    let whole = statement.refused.is_empty()
        || statement.refused.iter().any(|key| key == STATEMENT_REFUSAL);
    let mut named: BTreeSet<String> = BTreeSet::new();
    for key in &statement.refused {
        named.extend(construct_forms(key, &used, functions, links).0);
    }
    let refused_forms: BTreeSet<String> = match whole || named.is_empty() {
        true => forms.clone(),
        false => named,
    };
    StatementLinks {
        forms,
        refused_forms,
        unaccounted,
    }
}
