use std::collections::BTreeMap;

use crate::provenance::models::ProvenanceLog;

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
