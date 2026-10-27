//! `parser_dispatch_behavior_by_name_upper`: returns parser dispatch behavior by uppercased function name.

use crate::functions::dispatch::_helpers::lookups::PARSER_DISPATCH_BEHAVIOR_BY_NAME;
use crate::functions::dispatch::models::ParserDispatchBehavior;

/// Returns parser dispatch behavior by uppercased function name.
pub(crate) fn parser_dispatch_behavior_by_name_upper(
    upper_name: &str,
) -> Option<ParserDispatchBehavior> {
    PARSER_DISPATCH_BEHAVIOR_BY_NAME.get(upper_name).copied()
}
