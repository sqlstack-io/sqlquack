//! `collect_refusals`: every refusal in one statement, not only the first.

use serde_json::Value;
use sqlquack_core::parsing::models::SourceLines;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_syntax::Expression;

use crate::conversion::_helpers::queries::query::convert_query;
use crate::conversion::_helpers::support::collecting::{Collector, collected};
use crate::conversion::_helpers::support::locations::Locator;
use crate::conversion::_helpers::support::scope::Context;
use crate::conversion::_helpers::support::tree::strip_positions;
use crate::conversion::_helpers::support::typed_leaves::{ConvertedQuery, TypedLeaves};
use crate::conversion::main::convert_statement::convert_statement;
use crate::conversion::models::{CollectedRefusals, ConversionOptions, Schema};

/// The statement's refusals, converting past each refused node; its conversion is discarded.
pub fn collect_refusals(
    statement: &Expression,
    (schema, options, lines): (&Schema, &ConversionOptions, &SourceLines),
) -> CollectedRefusals {
    let converted = serde_json::to_value(statement)
        .map(|positioned| collected_conversion(&positioned, (schema, options, lines)));
    let mut guessed = false;
    let mut collected = match converted {
        Ok((result, found, typed)) => {
            guessed = found.events > 0;
            let mut collected = CollectedRefusals {
                refusals: found.refusals.into_iter().chain(typed).collect(),
                possibly_incomplete: found.incomplete,
                columns: None,
            };
            if let Ok((_, columns)) = &result {
                collected.columns = Some(columns.clone());
            }
            if let Err(refusal) = result {
                collected.possibly_incomplete = true;
                if !guessed || refusal.category == RefusalCategory::MissingRule {
                    collected.refusals.push(refusal);
                }
            }
            collected
        }
        Err(_) => CollectedRefusals::default(),
    };
    if collected.refusals.is_empty()
        && !guessed
        && let Err(refusal) = convert_statement(
            statement,
            (schema, options, lines),
            ProvenanceLog::default(),
        )
    {
        collected.refusals.push(refusal);
    }
    collected
}

/// The statement's conversion while collecting, and its refusal for typed leaves compared by text.
fn collected_conversion(
    positioned: &Value,
    (schema, options, lines): (&Schema, &ConversionOptions, &SourceLines),
) -> (ConvertedQuery, Collector, Option<Refusal>) {
    let tree = strip_positions(positioned);
    let locator = Locator::new(&tree, positioned, lines);
    let log = ProvenanceLog::default();
    let typed = TypedLeaves::default();
    let context = Context {
        log: &log,
        schema,
        options,
        ctes: Vec::new(),
        locator: &locator,
        typed: &typed,
    };
    let (result, found) = collected(|| convert_query(&tree, None, &context, true));
    let refusal = typed
        .refusal()
        .map(|refusal| locator.placed(refusal, &tree));
    let result = result.map(|(sql, columns)| (sql, typed.marked(columns)));
    (result, found, refusal)
}
