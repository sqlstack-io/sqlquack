//! Loading the policy and cases for evidence commands, and the caller's conversion choices.

use std::path::Path;

use sqlquack_convert::snowflake::analysis::main::caller_choices::caller_choices;
use sqlquack_convert::snowflake::analysis::models::ConversionChoices;
use sqlquack_convert::snowflake::conversion::models::ConversionOptions;
use sqlquack_convert::snowflake::session::main::builtin_conversion_options::builtin_conversion_options;
use sqlquack_convert::snowflake::session::main::confirm_utc_session::confirm_utc_session;
use sqlquack_convert::snowflake::session::main::conversion_options::conversion_options;
use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_policy::policy::constants::POLICY_FILE_NAME;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::Policy;

use crate::cli::models::{EvidencePaths, PolicyArguments};

pub(crate) struct Evidence {
    pub(crate) policy: Policy,
    pub(crate) cases: Vec<LoadedCase>,
}

pub(crate) fn load_evidence(paths: &EvidencePaths) -> Result<Evidence, String> {
    let policy = load_policy(&paths.policy).map_err(|error| error.to_string())?;
    let cases = load_cases(&paths.cases, &policy).map_err(|error| error.to_string())?;
    Ok(Evidence { policy, cases })
}

/// The `--policy` file, validated.
pub(crate) fn policy_file(path: &Path) -> Result<Policy, String> {
    if !path.is_file() {
        return Err(format!(
            "no policy file at {}; pass --policy with an existing file, or leave --policy out to \
             use the project's sqlquack.toml or, without one, the built-in policy, which refuses \
             every known difference but the clock and checks that only raise",
            path.display()
        ));
    }
    load_policy(path).map_err(|error| error.to_string())
}

/// The conversion options of `policy` under the caller's choices, or the built-in default.
pub(crate) fn divergence_options(
    policy: Option<&Policy>,
    arguments: &PolicyArguments,
) -> ConversionOptions {
    let Some(policy) = policy else {
        return builtin_conversion_options(arguments.strict, arguments.loud_divergences_only);
    };
    let options = conversion_options(policy, arguments.strict);
    let options = match arguments.utc_session {
        true => confirm_utc_session(options, policy),
        false => options,
    };
    caller_choices(options, policy, &conversion_choices(arguments))
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

/// The caller's flags as the engine-free conversion pipeline reads them, with the policy file its help names.
pub(crate) fn conversion_choices(arguments: &PolicyArguments) -> ConversionChoices {
    let policy_file = match (&arguments.policy, &arguments.project) {
        (Some(policy), _) => policy.clone(),
        (None, Some(project)) => project.join(POLICY_FILE_NAME),
        (None, None) => POLICY_FILE_NAME.into(),
    };
    ConversionChoices {
        policy_file: policy_file.display().to_string(),
        strict: arguments.strict,
        utc_session: arguments.utc_session,
        confirm_declared_offsets: arguments.confirm_declared_offsets,
        python_runtime: arguments.python_runtime,
        loud_divergences_only: arguments.loud_divergences_only,
    }
}
