use std::collections::BTreeMap;

use crate::provenance::models::{ProvenanceLog, ShapeArgument};

pub(crate) fn record_nested_rules() -> BTreeMap<String, Vec<String>> {
    let log = ProvenanceLog::default();
    log.building_block("duckdb.outside");
    {
        let _outer = log.rule("rule.outer");
        log.typing("snowflake.before_inner");
        {
            let _inner = log.rule("rule.inner");
            log.building_block("duckdb.inner_block");
        }
        log.building_block("duckdb.after_inner");
    }
    log.finish()
        .rule_uses
        .into_iter()
        .map(|(rule, uses)| (rule, uses.into_iter().collect()))
        .collect()
}

/// A GET of a column and a key converted twice and out of source order, with a facet.
pub(crate) fn record_get_shape() -> Vec<String> {
    let log = ProvenanceLog::default();
    log.enter("fn:GET", false);
    {
        let _rule = log.rule("rule.get");
        log.facet("form", "computed");
        for (node, offset, family) in [(2, 10, "text"), (1, 4, "variant"), (2, 10, "text")] {
            log.enter("node:column", false);
            log.leave(
                Some(ShapeArgument {
                    family: family.to_owned(),
                    literal: false,
                    source: None,
                }),
                (node, Some(offset)),
            );
        }
    }
    log.leave(Some(ShapeArgument::default()), (0, Some(0)));
    log.finish()
        .shapes
        .into_iter()
        .flat_map(|(rule, shapes)| {
            shapes.into_iter().map(move |shape| {
                let families: Vec<String> = shape
                    .arguments
                    .iter()
                    .map(|argument| argument.family.clone())
                    .collect();
                format!(
                    "{rule} {}({}) {:?}",
                    shape.construct,
                    families.join(", "),
                    shape.facets
                )
            })
        })
        .collect()
}
