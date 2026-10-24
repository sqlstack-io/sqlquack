//! Loading the policy and cases for evidence commands.

use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::Policy;

use crate::cli::models::EvidencePaths;

pub(crate) struct Evidence {
    pub(crate) policy: Policy,
    pub(crate) cases: Vec<LoadedCase>,
}

pub(crate) fn load_evidence(paths: &EvidencePaths) -> Result<Evidence, String> {
    let policy = load_policy(&paths.policy).map_err(|error| error.to_string())?;
    let cases = load_cases(&paths.cases, &policy).map_err(|error| error.to_string())?;
    Ok(Evidence { policy, cases })
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
