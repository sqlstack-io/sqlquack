//! `probed_constructs`: every construct a parser has the syntax for.

use crate::depth::_helpers::constructs::constructs;
use crate::depth::constants::DEPTH_LADDER;
use crate::depth::models::Construct;
use crate::formats::models::LocalParser;

/// The constructs `parser` is probed with, in report order.
pub fn probed_constructs(parser: LocalParser) -> Vec<Construct> {
    constructs()
        .into_iter()
        .filter(|construct| (construct.text)(parser, DEPTH_LADDER[0]).is_some())
        .collect()
}
