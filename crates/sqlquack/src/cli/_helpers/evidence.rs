//! Loading the policy and cases for evidence commands.

use std::path::Path;

use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::Policy;

use sqlquack_rules::conversion::models::ConversionOptions;

use crate::analysis::main::confirm_assumption::confirm_assumption;
use crate::analysis::main::refuse_value_divergences::refuse_value_divergences;
use crate::cli::models::{EvidencePaths, PolicyArguments};
use sqlquack_rules::conversion::constants::{DECLARED_OFFSET, PROJECT_UDF};
use sqlquack_rules::session::main::confirm_utc_session::confirm_utc_session;
use sqlquack_rules::session::main::conversion_options::conversion_options;

pub(crate) struct Evidence {
    pub(crate) policy: Policy,
    pub(crate) cases: Vec<LoadedCase>,
}

pub(crate) fn load_evidence(paths: &EvidencePaths) -> Result<Evidence, String> {
    let policy = load_policy(&paths.policy).map_err(|error| error.to_string())?;
    let cases = load_cases(&paths.cases, &policy).map_err(|error| error.to_string())?;
    Ok(Evidence { policy, cases })
}

/// The policy at `path`, or none when `builtin` asks for the default; a missing file is an error.
pub(crate) fn chosen_policy(path: &Path, builtin: bool) -> Result<Option<Policy>, String> {
    if builtin {
        return Ok(None);
    }
    if !path.is_file() {
        return Err(format!(
            "no policy file at {}; pass --policy <file>, or --builtin-policy to refuse every known \
             divergence",
            path.display()
        ));
    }
    load_policy(path)
        .map(Some)
        .map_err(|error| error.to_string())
}

/// The conversion options of `policy` under the caller's choices, or the built-in default.
pub(crate) fn divergence_options(
    policy: Option<&Policy>,
    arguments: &PolicyArguments,
) -> ConversionOptions {
    let Some(policy) = policy else {
        return ConversionOptions::default();
    };
    let options = conversion_options(policy, arguments.strict);
    let options = match arguments.utc_session {
        true => confirm_utc_session(options, policy),
        false => options,
    };
    caller_choices(options, policy, arguments)
}

/// `options` with the caller's confirmed assumptions, less value divergences if only loud ones.
pub(crate) fn caller_choices(
    options: ConversionOptions,
    policy: &Policy,
    arguments: &PolicyArguments,
) -> ConversionOptions {
    let options = match arguments.confirm_declared_offsets {
        true => confirm_assumption(options, policy, DECLARED_OFFSET),
        false => options,
    };
    let options = match arguments.python_runtime {
        true => confirm_assumption(options, policy, PROJECT_UDF),
        false => options,
    };
    match arguments.loud_divergences_only {
        true => refuse_value_divergences(options, policy),
        false => options,
    }
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
