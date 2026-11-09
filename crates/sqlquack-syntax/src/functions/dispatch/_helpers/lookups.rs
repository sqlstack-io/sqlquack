//! Functions dispatch: lookups.

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::functions::dispatch::constants::PARSER_DISPATCH_SPECS;
use crate::functions::dispatch::models::ParserDispatchBehavior;

/// Parser dispatch behavior map keyed by uppercased function name.
pub(crate) static PARSER_DISPATCH_BEHAVIOR_BY_NAME: LazyLock<
    HashMap<&'static str, ParserDispatchBehavior>,
> = LazyLock::new(|| {
    PARSER_DISPATCH_SPECS
        .iter()
        .map(|spec| (spec.name, spec.behavior))
        .collect()
});
