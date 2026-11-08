//! `probed_analyses`: every analysed construct a parser has an analysis profile for.

use crate::depth::_helpers::analysed::analysed_constructs;
use crate::depth::constants::ANALYSIS_LADDER;
use crate::depth::models::Construct;
use crate::formats::models::LocalParser;

/// The constructs `parser` is parsed and then analysed with, in report order.
pub fn probed_analyses(parser: LocalParser) -> Vec<Construct> {
    analysed_constructs()
        .into_iter()
        .filter(|construct| (construct.text)(parser, ANALYSIS_LADDER[0]).is_some())
        .collect()
}
