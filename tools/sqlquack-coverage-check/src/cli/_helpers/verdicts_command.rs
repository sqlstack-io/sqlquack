//! `corpus-verdicts`: the corpora's parse verdicts against the recorded ones.

use std::path::Path;
use std::process::ExitCode;

use crate::cli::_helpers::output::say;
use crate::verdicts::constants::SHOWN_CHANGES;
use crate::verdicts::main::changed_verdicts::changed_verdicts;
use crate::verdicts::main::current_verdicts::current_verdicts;
use crate::verdicts::models::{CorpusParse, Verdicts};

fn failed(problem: &str) -> ExitCode {
    say(&format!("FAILED, {problem}"));
    ExitCode::FAILURE
}

fn write_verdicts(path: &Path, verdicts: &Verdicts) -> Result<ExitCode, ExitCode> {
    let text: String = serde_json::to_string_pretty(verdicts)
        .map_err(|error| failed(&format!("the verdicts do not serialise: {error}")))?;
    std::fs::write(path, text + "\n")
        .map_err(|error| failed(&format!("{}: {error}", path.display())))?;
    say(&format!("recorded the verdicts in {}", path.display()));
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn corpus_verdicts(
    corpora: &Path,
    verdicts_path: &Path,
    update: bool,
) -> Result<ExitCode, ExitCode> {
    say(&format!(
        "parsing every statement under {}",
        corpora.display()
    ));
    let CorpusParse {
        verdicts: current,
        broken,
    } = current_verdicts(corpora).map_err(|problem| failed(&problem))?;
    for problem in broken.iter().take(SHOWN_CHANGES) {
        say(&format!("BROKEN {problem}"));
    }
    if !broken.is_empty() {
        return Err(failed(&format!(
            "{} statements break the lossless-token or located-parse invariant",
            broken.len()
        )));
    }
    let statements: usize = current
        .corpora
        .values()
        .flat_map(|by_dialect| by_dialect.values())
        .map(|verdicts| verdicts.statements)
        .sum();
    if update {
        return write_verdicts(verdicts_path, &current);
    }
    let recorded: Verdicts = std::fs::read_to_string(verdicts_path)
        .map_err(|error| error.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|error| error.to_string()))
        .map_err(|problem| failed(&format!("{}: {problem}", verdicts_path.display())))?;
    let changes: Vec<String> = changed_verdicts(&recorded, &current);
    for change in changes.iter().take(SHOWN_CHANGES) {
        say(&format!("CHANGED {change}"));
    }
    match (changes.is_empty(), recorded == current) {
        (true, true) => {
            say(&format!(
                "corpus verdicts: passed, {statements} verdicts as recorded"
            ));
            Ok(ExitCode::SUCCESS)
        }
        (true, false) => Err(failed(&format!(
            "{} differs from the current verdicts outside them; record it with `make corpus-verdicts-baseline`",
            verdicts_path.display()
        ))),
        (false, _) => Err(failed(&format!(
            "{} parse verdicts changed; review them and record them with `make corpus-verdicts-baseline`",
            changes.len()
        ))),
    }
}
