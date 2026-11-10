//! `builtin_conversion_options`: the built-in defaults: each known difference the evidence allows.

use crate::snowflake::analysis::main::confirm_assumption::confirm_assumption;
use crate::snowflake::conversion::models::ConversionOptions;
use crate::snowflake::session::constants::STRICT_FACTS;
use sqlquack_policy::policy::main::evidence_policy::evidence_policy;
use sqlquack_policy::policy::models::BuiltinDefault;

/// Each known difference the evidence allows by default; under `strict`, only the facts it counts.
pub fn builtin_conversion_options(strict: bool) -> ConversionOptions {
    let evidence = evidence_policy();
    evidence
        .divergences
        .iter()
        .filter(|(id, divergence)| match strict {
            true => STRICT_FACTS.contains(&id.as_str()),
            false => divergence.default == BuiltinDefault::Allow,
        })
        .fold(ConversionOptions::default(), |options, (id, _)| {
            confirm_assumption(options, evidence, id)
        })
}
