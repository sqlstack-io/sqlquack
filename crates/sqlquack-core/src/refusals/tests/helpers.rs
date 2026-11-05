use std::collections::BTreeMap;
use std::path::Path;

use crate::classification::main::classify::classify;
use crate::classification::models::Occurrence;
use crate::diagnostics::main::diagnose::diagnose;
use crate::parsing::main::parse_source::parse_source;
use crate::parsing::models::{SourceDialect, SourceLines};
use crate::refusals::main::refuse_constructs::refuse_constructs;
use crate::refusals::main::refuse_unparsed::refuse_unparsed;
use crate::refusals::main::snowflake_rejections::snowflake_rejections;
use crate::refusals::models::{Refusal, RefusalCategory};
use crate::refusals::tests::test_types::DeclaringPolicy;

fn occurrences(sql: &str) -> Vec<Occurrence> {
    parse_source(sql, SourceDialect::Snowflake)
        .expect("test SQL parses")
        .iter()
        .flat_map(|statement| classify(statement, &SourceLines::new(sql), SourceDialect::Snowflake))
        .collect()
}

fn refusal_matches(refusal: &Refusal, occurrence: &Occurrence) -> bool {
    refusal.construct.as_ref() == Some(&occurrence.construct)
        && refusal.location == occurrence.location
        && refusal.reason.contains(&occurrence.construct.key())
}

pub(crate) fn unmatched_refusals(sql: &str, inexpressible: &'static [&'static str]) -> usize {
    let found = occurrences(sql);
    let refusals = refuse_constructs(&found, &DeclaringPolicy { inexpressible });
    let matched = refusals
        .iter()
        .zip(&found)
        .filter(|(refusal, occurrence)| refusal_matches(refusal, occurrence))
        .count();
    refusals.len().max(found.len()) - matched
}

pub(crate) fn not_expressible_reasons(
    sql: &str,
    inexpressible: &'static [&'static str],
) -> Vec<String> {
    refuse_constructs(&occurrences(sql), &DeclaringPolicy { inexpressible })
        .into_iter()
        .filter(|refusal| refusal.category == RefusalCategory::NotExpressible)
        .map(|refusal| refusal.reason)
        .collect()
}

pub(crate) fn unparsed_refusal(sql: &str) -> Refusal {
    refuse_unparsed(
        &parse_source(sql, SourceDialect::Snowflake).expect_err("test SQL does not parse"),
        &SourceLines::new(sql),
    )
}

/// Every Snowflake rejection in `sql`, in order.
fn rejections(sql: &str) -> Vec<Refusal> {
    let lines = SourceLines::new(sql);
    parse_source(sql, SourceDialect::Snowflake)
        .expect("test SQL parses")
        .iter()
        .flat_map(|statement| snowflake_rejections(statement, (sql, &lines)))
        .collect()
}

/// The construct names of every Snowflake rejection in `sql`, in order.
pub(crate) fn rejection_names(sql: &str) -> Vec<String> {
    rejections(sql)
        .into_iter()
        .filter_map(|refusal| refusal.construct.map(|construct| construct.name))
        .collect()
}

/// The diagnostic code of every Snowflake rejection in `sql`, in order.
pub(crate) fn rejection_codes(sql: &str) -> Vec<&'static str> {
    diagnose(&rejections(sql), sql)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

/// The `NAME MAX` lines of the functions every catalogue call grammar bounds, sorted by name.
pub(crate) fn catalogue_function_arities() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../coverage/syntax/catalogue");
    let grammars: Vec<String> = std::fs::read_dir(root)
        .expect("the catalogue reads")
        .map(|entry| entry.expect("a catalogue file").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .map(|path| std::fs::read_to_string(path).expect("a catalogue file reads"))
        .map(|text| toml::from_str::<toml::Table>(&text).expect("a catalogue file parses"))
        .flat_map(|catalogue| array(catalogue.get("form")))
        .flat_map(|form| array(form.get("grammar")))
        .filter_map(|grammar| grammar.as_str().map(str::to_owned))
        .collect();
    let mut most: BTreeMap<String, Option<usize>> = BTreeMap::new();
    for (name, arguments) in grammars.iter().filter_map(|grammar| call_parts(grammar)) {
        let bound = arguments.and_then(most_in_grammar);
        most.entry(name)
            .and_modify(|known| *known = known.zip(bound).map(|(a, b)| a.max(b)))
            .or_insert(bound);
    }
    most.into_iter()
        .filter_map(|(name, bound)| bound.map(|bound| format!("{name} {bound}\n")))
        .collect()
}

/// The entries of a TOML array, or none.
fn array(value: Option<&toml::Value>) -> Vec<toml::Value> {
    value
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// A call grammar's function name and its argument text, which is absent when the call is unclosed.
fn call_parts(grammar: &str) -> Option<(String, Option<&str>)> {
    let (head, rest) = grammar.trim_start().split_once('(')?;
    let name = head.trim_end();
    let mut characters = name.chars();
    let starts = characters
        .next()
        .is_some_and(|first| first.is_ascii_uppercase() || first == '_');
    let continues = characters.all(|character| {
        character.is_ascii_uppercase() || character.is_ascii_digit() || "_.$".contains(character)
    });
    (starts && continues).then(|| (name.to_owned(), rest.trim_end().strip_suffix(')')))
}

/// The most arguments a grammar of plain `<argument>`s, `[ , ... ]` options and commas takes.
fn most_in_grammar(arguments: &str) -> Option<usize> {
    let spaced = grammar_spaced(arguments);
    let tokens: Vec<&str> = spaced.split_whitespace().collect();
    let known = tokens.iter().all(|token| GRAMMAR_TOKENS.contains(token));
    let balanced = tokens.iter().try_fold(0_usize, |depth, token| {
        (depth + usize::from(*token == "[")).checked_sub(usize::from(*token == "]"))
    }) == Some(0);
    let plain: Vec<&str> = tokens
        .iter()
        .copied()
        .filter(|token| *token != "[" && *token != "]")
        .collect();
    let alternating = plain.is_empty()
        || plain.first() == Some(&ARGUMENT)
            && plain.last() == Some(&ARGUMENT)
            && plain.windows(2).all(|pair| pair[0] != pair[1]);
    let count = plain.iter().filter(|token| **token == ARGUMENT).count();
    (known && balanced && alternating).then_some(count)
}

/// A grammar's arguments as `<>`, and its brackets and commas, each set apart by spaces.
fn grammar_spaced(text: &str) -> String {
    let mut parts = text.split('<');
    let first = parts.next().unwrap_or_default().replace('>', " ");
    let arguments: String = parts
        .map(|part| {
            part.split_once('>').map_or_else(
                || format!(" <{}", part.replace('>', " ")),
                |(_, rest)| format!(" {ARGUMENT} {}", rest.replace('>', " ")),
            )
        })
        .collect();
    (first + &arguments)
        .replace('[', " [ ")
        .replace(']', " ] ")
        .replace(',', " , ")
}

/// How [`grammar_spaced`] writes an argument.
const ARGUMENT: &str = "<>";
/// The tokens a grammar of plain arguments holds.
const GRAMMAR_TOKENS: [&str; 4] = [ARGUMENT, "[", "]", ","];
