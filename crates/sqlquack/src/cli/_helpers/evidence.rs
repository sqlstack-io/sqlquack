//! Loading the policy and cases for evidence commands, and SQ0008's help for refused divergences.

use std::collections::BTreeMap;
use std::path::Path;

use sqlquack_core::diagnostics::models::Diagnostic;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_evidence::cases::main::load_cases::load_cases;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_policy::policy::constants::{POLICY_FILE_NAME, SESSION_TIME_ZONE, TIMEZONE_PARAMETER};
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::{DivergenceFailure, DivergenceMode, Policy};

use sqlquack_rules::conversion::models::ConversionOptions;

use crate::analysis::main::assumption_id::assumption_id;
use crate::analysis::main::confirm_assumption::confirm_assumption;
use crate::analysis::main::refuse_value_divergences::refuse_value_divergences;
use crate::cli::models::{EvidencePaths, PolicyArguments};
use sqlquack_policy::policy::constants::{
    CURRENT_TIME, DECLARED_OFFSET, NUMERIC_RANGE, PROJECT_UDF, SESSION_TIMEZONE_UTC,
    TIMESTAMP_RANGE, TIMESTAMP_TZ_SUBMICRO,
};
use sqlquack_rules::session::main::builtin_conversion_options::builtin_conversion_options;
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

/// The `--policy` file, validated.
pub(crate) fn policy_file(path: &Path) -> Result<Policy, String> {
    if !path.is_file() {
        return Err(format!(
            "no policy file at {}; pass --policy with an existing file, or leave --policy out to \
             use the project's sqlquack.toml or, without one, the built-in policy, which refuses \
             every known divergence but the clock and checks that only raise",
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

/// `diagnostics`, each SQ0008 one's help naming its divergence and the fixes that work here.
pub(crate) fn with_divergence_help(
    refusals: &[Refusal],
    diagnostics: Vec<Diagnostic>,
    (policy, arguments, parameters): (Option<&Policy>, &PolicyArguments, &BTreeMap<String, String>),
) -> Vec<Diagnostic> {
    refusals
        .iter()
        .zip(diagnostics)
        .map(|(refusal, mut diagnostic)| {
            if let Some(id) = refusal.divergence() {
                diagnostic.help = Some(divergence_help(
                    assumption_id(id),
                    (policy, arguments, parameters),
                ));
            }
            diagnostic
        })
        .collect()
}

/// The fixes for divergence `id`, given the policy, the caller's flags and session parameters.
fn divergence_help(
    id: &str,
    (policy, arguments, parameters): (Option<&Policy>, &PolicyArguments, &BTreeMap<String, String>),
) -> String {
    const LOUD: [&str; 4] = [
        JSON_FIDELITY,
        NUMERIC_RANGE,
        TIMESTAMP_RANGE,
        TIMESTAMP_TZ_SUBMICRO,
    ];
    let Some(policy) = policy else {
        return match [CURRENT_TIME, SESSION_TIMEZONE_UTC].contains(&id) {
            true => format!(
                "the built-in policy allows `{id}` only for CURRENT_DATE() and \
                 CURRENT_TIMESTAMP(), and DATEADD, DATEDIFF, TIMESTAMPDIFF, TIMEDIFF, COALESCE, \
                 IFNULL, NVL and comparisons of a TIMESTAMP_LTZ or of a TIMESTAMP_TZ with text \
                 that has no offset, in a UTC session, without --strict or \
                 --loud-divergences-only; drop those flags, or pass --policy with a policy file \
                 that allows `{id}` here, such as the repository's sqlquack.toml"
            ),
            false if LOUD.contains(&id) => format!(
                "the built-in policy allows `{id}` only for {}, where DuckDB raises an error \
                 rather than return a different value, and not under --strict; drop --strict, or \
                 pass --policy with a policy file that allows `{id}` here, such as the \
                 repository's sqlquack.toml",
                builtin_conversion_options(false, false)
                    .divergences
                    .get(id)
                    .map(|allowed| allowed.constructs.join(", "))
                    .unwrap_or_default()
            ),
            false => format!(
                "the built-in policy refuses every known divergence but the clock, \
                 arithmetic's 38-digit and timestamp range errors, VARIANT reads' JSON guards \
                 and the sub-microsecond error of a naive timestamp joined with a local one; \
                 pass --policy with a policy file that allows `{id}`, such as the repository's \
                 sqlquack.toml"
            ),
        };
    };
    let file = match (&arguments.policy, &arguments.project) {
        (Some(policy), _) => policy.clone(),
        (None, Some(project)) => project.join(POLICY_FILE_NAME),
        (None, None) => POLICY_FILE_NAME.into(),
    };
    let file = file.display();
    let Some(divergence) = policy.divergences.get(id) else {
        return format!(
            "{file} has no [divergences.{id}]; copy it from the repository's sqlquack.toml"
        );
    };
    let zone = parameters
        .get(TIMEZONE_PARAMETER)
        .filter(|zone| *zone != SESSION_TIME_ZONE);
    if let (SESSION_TIMEZONE_UTC, Some(zone)) = (id, zone) {
        return format!(
            "`{id}` holds only in a UTC session, and TIMEZONE = {zone}; convert for a UTC \
             session (--session TIMEZONE=UTC)"
        );
    }
    let mut fixes: Vec<String> = Vec::new();
    let confirmation = match id {
        SESSION_TIMEZONE_UTC => Some(("utc_session", "--utc-session", arguments.utc_session)),
        DECLARED_OFFSET => Some((
            "declared_offsets",
            "--confirm-declared-offsets",
            arguments.confirm_declared_offsets,
        )),
        PROJECT_UDF => Some((
            "python_runtime",
            "--python-runtime",
            arguments.python_runtime,
        )),
        _ => None,
    };
    let confirmable = match confirmation {
        Some((key, flag, false)) => {
            fixes.push(format!(
                "confirm its assumption: add `{key} = true` under [project.confirm] in {file}, or \
                 pass {flag} for one run"
            ));
            true
        }
        Some((_, _, true)) => true,
        None => false,
    };
    let mode = match arguments.strict {
        true => divergence.strict,
        false => divergence.default,
    };
    if mode == DivergenceMode::Refuse && !confirmable {
        let keys = match (arguments.strict, divergence.default) {
            (true, DivergenceMode::Warn) => {
                fixes.push("drop --strict".to_owned());
                "strict"
            }
            (true, DivergenceMode::Refuse) => "default and strict",
            (false, _) => "default",
        };
        fixes.push(format!(
            "set {keys} = \"warn\" under [divergences.{id}] in {file}"
        ));
    }
    let value = divergence.failure == DivergenceFailure::Value;
    match (arguments.loud_divergences_only && value, fixes.is_empty()) {
        (true, true) => format!(
            "`{id}` can return a different value rather than raise, which \
             --loud-divergences-only refuses; drop that flag"
        ),
        (true, false) => format!(
            "drop --loud-divergences-only, since `{id}` can return a different value rather \
             than raise, and {}",
            fixes.join(", or ")
        ),
        (false, false) => format!("to rely on `{id}`: {}", fixes.join(", or ")),
        (false, true) => format!("allow `{id}` under [divergences.{id}] in {file}"),
    }
}
