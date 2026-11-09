//! `with_divergence_help`: SQ0008 diagnostics with help naming the fixes that work for the caller.

use std::collections::BTreeMap;

use sqlquack_core::diagnostics::models::Diagnostic;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_policy::policy::constants::{
    CURRENT_TIME, DECLARED_OFFSET, NUMERIC_RANGE, PROJECT_UDF, SESSION_TIME_ZONE,
    SESSION_TIMEZONE_UTC, TIMESTAMP_RANGE, TIMESTAMP_TZ_SUBMICRO, TIMEZONE_PARAMETER,
};
use sqlquack_policy::policy::models::{DivergenceFailure, DivergenceMode, Policy};
use sqlquack_rules::session::main::builtin_conversion_options::builtin_conversion_options;

use crate::analysis::main::assumption_id::assumption_id;
use crate::analysis::models::ConversionChoices;

/// `diagnostics`, each SQ0008 one's help naming its divergence and the fixes that work here.
pub fn with_divergence_help(
    refusals: &[Refusal],
    diagnostics: Vec<Diagnostic>,
    (policy, choices, parameters): (
        Option<&Policy>,
        &ConversionChoices,
        &BTreeMap<String, String>,
    ),
) -> Vec<Diagnostic> {
    refusals
        .iter()
        .zip(diagnostics)
        .map(|(refusal, mut diagnostic)| {
            if let Some(id) = refusal.divergence() {
                diagnostic.help = Some(divergence_help(
                    assumption_id(id),
                    (policy, choices, parameters),
                ));
            }
            diagnostic
        })
        .collect()
}

/// The fixes for divergence `id`, given the policy, the caller's flags and session parameters.
fn divergence_help(
    id: &str,
    (policy, choices, parameters): (
        Option<&Policy>,
        &ConversionChoices,
        &BTreeMap<String, String>,
    ),
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
    let file = &choices.policy_file;
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
        SESSION_TIMEZONE_UTC => Some(("utc_session", "--utc-session", choices.utc_session)),
        DECLARED_OFFSET => Some((
            "declared_offsets",
            "--confirm-declared-offsets",
            choices.confirm_declared_offsets,
        )),
        PROJECT_UDF => Some(("python_runtime", "--python-runtime", choices.python_runtime)),
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
    let mode = match choices.strict {
        true => divergence.strict,
        false => divergence.default,
    };
    if mode == DivergenceMode::Refuse && !confirmable {
        let keys = match (choices.strict, divergence.default) {
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
    match (choices.loud_divergences_only && value, fixes.is_empty()) {
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
