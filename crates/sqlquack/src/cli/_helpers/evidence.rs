//! Loading the evidence policy and cases for evidence commands, and the caller's conversion choices.

use std::path::Path;

use sqlquack_convert::snowflake::analysis::models::ConversionChoices;
use sqlquack_convert::snowflake::conversion::models::ConversionOptions;
use sqlquack_convert::snowflake::session::main::configured_options::configured_options;
use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_policy::config::models::UserConfig;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::Policy;

use crate::cli::models::{EvidencePaths, PolicyArguments};

pub(crate) struct Evidence {
    pub(crate) policy: Policy,
    pub(crate) cases: Vec<LoadedCase>,
}

pub(crate) fn load_evidence(paths: &EvidencePaths) -> Result<Evidence, String> {
    let policy = load_policy(&paths.evidence).map_err(|error| error.to_string())?;
    let cases = load_cases(&paths.cases, &policy).map_err(|error| error.to_string())?;
    Ok(Evidence { policy, cases })
}

/// The known differences the caller's choices allow, with the user's `[allow]` applied.
pub(crate) fn divergence_options(choices: &ConversionChoices) -> ConversionOptions {
    configured_options(choices)
}

/// Parses `10%` and similar into a percentage between 0 and 100.
pub(crate) fn parse_percent(sample: &str) -> Result<f64, String> {
    let invalid = || format!("--sample needs a percentage such as 10%, not `{sample}`");
    let number = sample.trim().strip_suffix('%').ok_or_else(invalid)?;
    match number.trim().parse::<f64>() {
        Ok(percent) if (0.0..=100.0).contains(&percent) => Ok(percent),
        Ok(_) | Err(_) => Err(invalid()),
    }
}

/// The caller's flags and configuration as conversion reads them, naming the file as the caller gave it.
pub(crate) fn conversion_choices(
    arguments: &PolicyArguments,
    (config, config_file): (&UserConfig, Option<&Path>),
) -> ConversionChoices {
    ConversionChoices {
        config_file: config_file.map(|path| path.display().to_string()),
        allow: config.allow.clone(),
        strict: arguments.strict,
        python_runtime: arguments.python_runtime || config.project.python_runtime,
    }
}
