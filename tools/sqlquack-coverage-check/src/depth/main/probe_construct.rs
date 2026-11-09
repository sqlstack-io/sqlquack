//! `probe_construct`: one construct parsed at each ladder depth until it first fails.

use crate::check::models::{LocalFailure, LocalVerdict};
use crate::depth::models::{Construct, ConstructResult};
use crate::formats::models::LocalParser;

/// Where `construct` first crashes, panics or times out in `verdict` up `ladder`; refusals are clean.
pub fn probe_construct(
    (parser, ladder): (LocalParser, &[usize]),
    construct: &Construct,
    verdict: &mut dyn FnMut(&str) -> LocalVerdict,
) -> ConstructResult {
    let failed_at: Option<(usize, LocalFailure)> = ladder.iter().find_map(|&depth| {
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
