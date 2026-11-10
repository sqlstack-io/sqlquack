use std::path::Path;

use crate::known_differences::constants::KNOWN_DIFFERENCES;
use crate::policy::constants::POLICY_FILE_NAME;
use crate::policy::main::load_policy::load_policy;
use crate::policy::models::{DivergenceFailure, Policy};

const INTERNAL_WORDS: [&str; 10] = [
    "fn:",
    "node:",
    "cast:",
    "join:",
    "clause:",
    "syntax:",
    "loud",
    "assumption",
    "divergen",
    "--",
];

pub(crate) fn repository_policy() -> Policy {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(POLICY_FILE_NAME);
    load_policy(&path).expect("the repository policy is valid")
}

pub(crate) fn entries_unlike_policy() -> Vec<String> {
    let policy = repository_policy();
    let described: Vec<(&str, DivergenceFailure)> = KNOWN_DIFFERENCES
        .iter()
        .map(|entry| (entry.name, entry.failure))
        .collect();
    let declared: Vec<(&str, DivergenceFailure)> = policy
        .divergences
        .iter()
        .map(|(name, divergence)| (name.as_str(), divergence.failure))
        .collect();
    let missing = declared
        .iter()
        .filter(|pair| described.iter().filter(|entry| entry == pair).count() != 1)
        .map(|(name, failure)| format!("{name} ({failure:?}) needs exactly one entry"));
    let unknown = described
        .iter()
        .filter(|pair| !declared.contains(pair))
        .map(|(name, failure)| format!("{name} ({failure:?}) is no divergence of the policy"));
    missing.chain(unknown).collect()
}

pub(crate) fn internal_names_in_text() -> Vec<String> {
    let names: Vec<String> = repository_policy()
        .session_profiles
        .into_keys()
        .chain(KNOWN_DIFFERENCES.iter().map(|entry| entry.name.to_owned()))
        .filter(|name| name.contains(['_', '-']))
        .chain(["`".to_owned()])
        .collect();
    KNOWN_DIFFERENCES
        .iter()
        .flat_map(|entry| {
            [
                entry.title,
                entry.differs,
                entry.fine_when,
                entry.wrong_when,
            ]
            .into_iter()
            .flat_map(|text| {
                let lower = text.to_lowercase();
                INTERNAL_WORDS
                    .iter()
                    .filter(|word| lower.contains(*word))
                    .map(|word| (*word).to_owned())
                    .chain(
                        names
                            .iter()
                            .filter(|name| text.contains(name.as_str()))
                            .cloned(),
                    )
                    .collect::<Vec<_>>()
            })
            .map(|found| format!("{}: {found}", entry.name))
        })
        .collect()
}
