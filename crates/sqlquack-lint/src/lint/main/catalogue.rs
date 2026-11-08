use crate::lint::_helpers::engine::rule_metadata;
use crate::lint::models::LintRuleMetadata;

pub fn catalogue() -> Vec<LintRuleMetadata> {
    rule_metadata()
}
