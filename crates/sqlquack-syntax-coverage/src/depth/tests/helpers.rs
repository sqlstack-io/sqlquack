use crate::check::models::{LocalFailure, LocalVerdict};
use crate::depth::models::{Construct, ConstructResult, KnownFailure};
use crate::formats::models::LocalParser;

/// One parenthesis per level, so a verdict can read the depth from the text's length.
pub(crate) const PARENTHESES: Construct = Construct {
    name: "parentheses",
    text: |_, depth| Some(format!("{}1{}", "(".repeat(depth), ")".repeat(depth))),
};

pub(crate) fn failed(failure: LocalFailure) -> LocalVerdict {
    LocalVerdict::Failed { failure }
}

pub(crate) fn accepted() -> LocalVerdict {
    LocalVerdict::Accepted {
        opaque: false,
        tree_sha256: None,
    }
}

/// `deep` for texts longer than `threshold` bytes, `shallow` for the rest.
pub(crate) fn by_length(
    threshold: usize,
    deep: &LocalVerdict,
    shallow: &LocalVerdict,
    sql: &str,
) -> LocalVerdict {
    [shallow, deep][usize::from(sql.len() > threshold)].clone()
}

pub(crate) fn result(
    parser: LocalParser,
    construct: &str,
    failed_at: Option<(usize, LocalFailure)>,
) -> ConstructResult {
    ConstructResult {
        parser,
        construct: construct.to_owned(),
        failed_at,
    }
}

pub(crate) fn entry(
    parser: LocalParser,
    construct: &str,
    profile: &str,
    depth: usize,
) -> KnownFailure {
    KnownFailure {
        parser,
        construct: construct.to_owned(),
        profile: profile.to_owned(),
        depth,
        failure: LocalFailure::Crash,
        owner: "sqlquack-syntax-postgres".to_owned(),
    }
}

/// The `parser construct` each report line is about.
pub(crate) fn subjects(lines: &[String]) -> Vec<&str> {
    lines
        .iter()
        .map(|line| line.split(':').next().unwrap_or_default())
        .collect()
}
