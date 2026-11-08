//! Nested constructs, the allowlist of known failures, and what one probe run found.

use serde::{Deserialize, Serialize};

use crate::check::models::LocalFailure;
use crate::formats::models::LocalParser;

/// One construct nested to a depth: its name, and its text in a parser that has that syntax.
#[derive(Clone, Copy, Debug)]
pub struct Construct {
    pub name: &'static str,
    /// The SQL text at a depth, or none when the parser has no such syntax.
    pub text: fn(LocalParser, usize) -> Option<String>,
}

/// `coverage/deep-nesting.toml`: every failure the probe tolerates until its owner fixes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Allowlist {
    #[serde(default)]
    pub known_failures: Vec<KnownFailure>,
}

/// A construct that crashes or hangs in one parser and build profile today.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KnownFailure {
    pub parser: LocalParser,
    pub construct: String,
    /// The Cargo profile the probe was built with: `gate` or `dev`.
    pub profile: String,
    /// The shallowest ladder depth at which the construct fails.
    pub depth: usize,
    pub failure: LocalFailure,
    /// The crate, and the part of it, whose fix removes this entry.
    pub owner: String,
}

/// Where one construct first failed in one parser, if anywhere.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConstructResult {
    pub parser: LocalParser,
    pub construct: String,
    /// The shallowest failing ladder depth and how it failed; none when every depth ends cleanly.
    pub failed_at: Option<(usize, LocalFailure)>,
}

/// A probe run held against the allowlist; the gate passes only when every list is empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DepthReport {
    /// Failures no entry names, or that an entry names at another depth or failure.
    pub unlisted: Vec<String>,
    /// Entries for this profile whose construct now ends cleanly at every depth.
    pub stale: Vec<String>,
    /// Entries naming a construct the probe does not run, or naming one twice.
    pub problems: Vec<String>,
}
