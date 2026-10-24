//! Loading the policy and cases for evidence commands.

use std::io::Write;

use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::Policy;

use crate::cli::models::EvidencePaths;

pub(crate) struct Evidence {
    pub(crate) policy: Policy,
    pub(crate) cases: Vec<LoadedCase>,
}

pub(crate) fn load_evidence(
    command: &str,
    paths: &EvidencePaths,
    stderr: &mut dyn Write,
) -> Option<Evidence> {
    let _ = writeln!(stderr, "sqlquack {command}: loading the policy and cases…");
    let policy = load_policy(&paths.policy)
        .map_err(|error| writeln!(stderr, "sqlquack {command}: failed: {error}"))
        .ok()?;
    let cases = load_cases(&paths.cases, &policy)
        .map_err(|error| writeln!(stderr, "sqlquack {command}: failed: {error}"))
        .ok()?;
    let _ = writeln!(stderr, "sqlquack {command}: loaded {} cases", cases.len());
    Some(Evidence { policy, cases })
}
