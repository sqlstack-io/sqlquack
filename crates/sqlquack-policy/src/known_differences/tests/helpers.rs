use crate::known_differences::constants::KNOWN_DIFFERENCES;
use crate::policy::main::evidence_policy::evidence_policy;
use crate::policy::models::{BuiltinDefault, DivergenceFailure, Policy};

const ERROR_ENDING: &str = ", never a different value.";

const VALUE_ENDING: &str = ", with no error.";

const ENDINGS: [(DivergenceFailure, &str); 2] = [
    (DivergenceFailure::Loud, ERROR_ENDING),
    (DivergenceFailure::Value, VALUE_ENDING),
];

/// The known differences the built-in defaults refuse (D806): every other one is allowed.
const REFUSED_BY_DEFAULT: [&str; 5] = [
    "hash",
    "project_udf",
    "sequences",
    "source_variant_json",
    "variant_chain_type",
];

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
    evidence_policy().clone()
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

pub(crate) fn entries_off_style() -> Vec<String> {
    KNOWN_DIFFERENCES
        .iter()
        .flat_map(|entry| {
            let ending = ENDINGS
                .iter()
                .find(|(failure, _)| *failure == entry.failure)
                .map_or("", |(_, ending)| *ending);
            [
                (sentences(entry.differs) > 2).then_some("differs has more than two sentences"),
                (sentences(entry.fine_when) != 1).then_some("fine_when is not one sentence"),
                (sentences(entry.wrong_when) != 1).then_some("wrong_when is not one sentence"),
                (!entry.wrong_when.ends_with(ending))
                    .then_some("wrong_when does not end as its failure says"),
                (entry.failure == DivergenceFailure::Loud
                    && !entry
                        .wrong_when
                        .contains("DuckDB raises an error where Snowflake"))
                .then_some("wrong_when does not say DuckDB raises an error where Snowflake"),
            ]
            .into_iter()
            .flatten()
            .map(|problem| format!("{}: {problem}", entry.name))
        })
        .collect()
}

pub(crate) fn notes_off_style() -> Vec<String> {
    KNOWN_DIFFERENCES
        .iter()
        .flat_map(|entry| {
            let notes = entry.notes();
            let shown = format!("{}\n{}", entry.summary(), notes.join("\n"));
            let refers_back = [entry.fine_when, entry.wrong_when].iter().any(|text| {
                text.split("such ")
                    .skip(1)
                    .any(|after| !after.starts_with("as "))
            });
            let unlabelled = notes.iter().filter(|note| {
                let sentence = ["fine when: ", "wrong when: "]
                    .iter()
                    .find_map(|label| note.strip_prefix(label))
                    .unwrap_or(note);
                !(sentence.starts_with(char::is_uppercase) && sentence.ends_with('.'))
            });
            [
                (!shown.contains(entry.fine_when)).then_some("fine_when is not shown whole"),
                (!shown.contains(entry.wrong_when)).then_some("wrong_when is not shown whole"),
                (shown.matches(". ").count() + shown.matches(".\n").count()
                    < sentences(entry.differs) - 1)
                    .then_some("part of differs is not shown"),
                (refers_back && notes.len() < 3)
                    .then_some("\"such\" refers to a sentence the diagnostic does not show"),
            ]
            .into_iter()
            .flatten()
            .map(str::to_owned)
            .chain(unlabelled.map(|note| format!("not a whole sentence: {note}")))
            .map(|problem| format!("{}: {problem}", entry.name))
            .collect::<Vec<String>>()
        })
        .collect()
}

fn sentences(text: &str) -> usize {
    text.matches(". ").count() + usize::from(text.ends_with('.'))
}

pub(crate) fn defaults_unlike_the_rule() -> Vec<String> {
    repository_policy()
        .divergences
        .iter()
        .filter(|(name, divergence)| {
            let refused = REFUSED_BY_DEFAULT.contains(&name.as_str());
            (divergence.default == BuiltinDefault::Refuse) != refused
        })
        .map(|(name, divergence)| format!("{name} defaults to {:?}", divergence.default))
        .collect()
}
