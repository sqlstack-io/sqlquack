//! `compare_allowlist`: a probe run against the known failures of one build profile.

use std::collections::BTreeSet;
use std::fmt::Debug;

use crate::depth::models::{Allowlist, ConstructResult, DepthReport, KnownFailure};

/// A one-word parser or failure as the allowlist writes it.
fn name<T: Debug>(value: T) -> String {
    format!("{value:?}").to_lowercase()
}

fn listing(entry: &KnownFailure) -> String {
    format!(
        "{} at depth {} (owner: {})",
        name(entry.failure),
        entry.depth,
        entry.owner
    )
}

/// Unlisted failures, stale entries for `profile`, and malformed entries of any profile.
pub fn compare_allowlist(
    results: &[ConstructResult],
    allowlist: &Allowlist,
    profile: &str,
) -> DepthReport {
    let mut report: DepthReport = DepthReport::default();
    let probed: BTreeSet<(String, String)> = results
        .iter()
        .map(|result| (name(result.parser), result.construct.clone()))
        .collect();
    let mut seen: BTreeSet<(String, String, String)> = BTreeSet::new();
    for entry in &allowlist.known_failures {
        let (parser, construct, entry_profile) = (
            name(entry.parser),
            entry.construct.clone(),
            entry.profile.clone(),
        );
        if !probed.contains(&(parser.clone(), construct.clone())) {
            report.problems.push(format!(
                "{parser} {construct}: listed, but the probe runs no such construct in {parser}"
            ));
        }
        if !seen.insert((parser.clone(), construct.clone(), entry_profile.clone())) {
            report.problems.push(format!(
                "{parser} {construct}: listed twice for profile {entry_profile}"
            ));
        }
    }
    for result in results {
        let (parser, construct) = (name(result.parser), &result.construct);
        let entry: Option<&KnownFailure> = allowlist.known_failures.iter().find(|entry| {
            entry.parser == result.parser
                && entry.construct == result.construct
                && entry.profile == profile
        });
        match (result.failed_at, entry) {
            (Some((depth, failure)), None) => report.unlisted.push(format!(
                "{parser} {construct}: {} at depth {depth}, and no entry lists it",
                name(failure)
            )),
            (Some((depth, failure)), Some(entry))
                if (depth, failure) != (entry.depth, entry.failure) =>
            {
                report.unlisted.push(format!(
                    "{parser} {construct}: {} at depth {depth}, but listed as {}; update the entry",
                    name(failure),
                    listing(entry)
                ));
            }
            (None, Some(entry)) => report.stale.push(format!(
                "{parser} {construct}: listed as {}, but it now ends cleanly at every depth; remove the entry",
                listing(entry)
            )),
            _ => {}
        }
    }
    report
}
