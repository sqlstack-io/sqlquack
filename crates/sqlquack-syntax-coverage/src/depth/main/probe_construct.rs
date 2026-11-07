//! `probe_construct`: one construct parsed at each ladder depth until it first fails.

use crate::check::models::{LocalFailure, LocalVerdict};
use crate::depth::constants::DEPTH_LADDER;
use crate::depth::models::{Construct, ConstructResult};
use crate::formats::models::LocalParser;

/// Where `construct` first crashes, panics or times out in `verdict`; a refusal ends cleanly.
pub fn probe_construct(
    parser: LocalParser,
    construct: &Construct,
    verdict: &mut dyn FnMut(&str) -> LocalVerdict,
) -> ConstructResult {
    let failed_at: Option<(usize, LocalFailure)> = DEPTH_LADDER.iter().find_map(|&depth| {
        let text: String = (construct.text)(parser, depth)?;
        match verdict(&text) {
            LocalVerdict::Failed {
                failure:
                    failure @ (LocalFailure::Crash | LocalFailure::Panic | LocalFailure::Timeout),
            } => Some((depth, failure)),
            _ => None,
        }
    });
    ConstructResult {
        parser,
        construct: construct.name.to_owned(),
        failed_at,
    }
}
