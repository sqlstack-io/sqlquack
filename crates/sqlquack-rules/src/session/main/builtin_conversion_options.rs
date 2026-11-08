//! `builtin_conversion_options`: the built-in default's options: the clock and loud checks.

use crate::conversion::models::{AllowedDivergence, ConversionOptions};
use crate::session::constants::{
    BUILTIN_CLOCK_CONSTRUCTS, BUILTIN_CLOCK_REASON, BUILTIN_CLOCK_ZONE, BUILTIN_JSON_CONSTRUCTS,
    BUILTIN_JSON_REASON, BUILTIN_NUMERIC_CONSTRUCTS, BUILTIN_NUMERIC_REASON,
    BUILTIN_SUBMICRO_CONSTRUCTS, BUILTIN_SUBMICRO_REASON, BUILTIN_TIMESTAMP_CONSTRUCTS,
    BUILTIN_TIMESTAMP_REASON, BUILTIN_ZONE_CONSTRUCTS,
};
use sqlquack_duckdb_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_policy::policy::constants::{
    CURRENT_TIME, NUMERIC_RANGE, SESSION_TIMEZONE_UTC, TIMESTAMP_RANGE, TIMESTAMP_TZ_SUBMICRO,
};

/// The built-in default's warned divergences: none if `strict`, and no clock if `loud_only`.
pub fn builtin_conversion_options(strict: bool, loud_only: bool) -> ConversionOptions {
    let mut options = ConversionOptions::default();
    if strict {
        return options;
    }
    let loud = [
        (
            JSON_FIDELITY,
            allowed(&BUILTIN_JSON_CONSTRUCTS, BUILTIN_JSON_REASON, false),
        ),
        (
            NUMERIC_RANGE,
            allowed(&BUILTIN_NUMERIC_CONSTRUCTS, BUILTIN_NUMERIC_REASON, false),
        ),
        (
            TIMESTAMP_RANGE,
            allowed(
                &BUILTIN_TIMESTAMP_CONSTRUCTS,
                BUILTIN_TIMESTAMP_REASON,
                false,
            ),
        ),
        (
            TIMESTAMP_TZ_SUBMICRO,
            allowed(&BUILTIN_SUBMICRO_CONSTRUCTS, BUILTIN_SUBMICRO_REASON, false),
        ),
    ];
    let clock = [
        (
            CURRENT_TIME,
            allowed(&BUILTIN_CLOCK_CONSTRUCTS, BUILTIN_CLOCK_REASON, false),
        ),
        (
            SESSION_TIMEZONE_UTC,
            allowed(&BUILTIN_ZONE_CONSTRUCTS, BUILTIN_CLOCK_ZONE, true),
        ),
    ];
    let clock = match loud_only {
        true => Vec::new(),
        false => clock.to_vec(),
    };
    for (divergence, allowance) in loud.into_iter().chain(clock) {
        options.divergences.insert(divergence.to_owned(), allowance);
    }
    options
}

/// An allowance at `constructs`, warned with `reason`, an assumption where `assumption`.
fn allowed(constructs: &[&str], reason: &str, assumption: bool) -> AllowedDivergence {
    AllowedDivergence {
        constructs: constructs
            .iter()
            .map(|construct| (*construct).to_owned())
            .collect(),
        reason: reason.to_owned(),
        assumption,
    }
}
