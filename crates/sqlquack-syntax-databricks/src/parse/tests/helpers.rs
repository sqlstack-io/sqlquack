use serde_json::Value;

use crate::parse::_helpers::tree::reading::{Reading, parse_as};
use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::ExpectedVerdict;

/// How many different trees, spans aside, the strict parser gives `written`.
pub(crate) fn distinct_trees(written: &[&str], description: &str) -> usize {
    let trees: Vec<Value> = written
        .iter()
        .map(|sql| {
            let parsed = parse_strict(sql).expect(description);
            without_spans(serde_json::to_value(&parsed.statements).expect(description))
        })
        .collect();
    trees
        .iter()
        .enumerate()
        .filter(|(at, tree)| !trees[..*at].contains(tree))
        .count()
}

/// `value` without its source spans, which differ between spellings of one form.
fn without_spans(mut value: Value) -> Value {
    value.as_object_mut().into_iter().for_each(|fields| {
        fields.remove("span");
        fields
            .values_mut()
            .for_each(|field| *field = without_spans(field.take()));
    });
    value
        .as_array_mut()
        .into_iter()
        .flatten()
        .for_each(|item| *item = without_spans(item.take()));
    value
}

/// The verdict `parse` gives `sql`.
pub(crate) fn verdict(sql: &str) -> ExpectedVerdict {
    parse_as(sql, Reading::Trees)
        .map(|parsed| ExpectedVerdict::Accepted {
            opaque: !parsed.gaps.is_empty(),
        })
        .unwrap_or_else(|error| ExpectedVerdict::Rejected {
            syntax: error.syntax,
        })
}
