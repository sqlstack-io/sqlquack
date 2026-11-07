//! `builtin_conversion_options`: the built-in default's options, which allow only the clock.

use crate::conversion::models::{AllowedDivergence, ConversionOptions};
use crate::session::constants::{
    BUILTIN_CLOCK_CONSTRUCTS, BUILTIN_CLOCK_REASON, BUILTIN_CLOCK_ZONE,
};
use sqlquack_policy::policy::constants::{CURRENT_TIME, SESSION_TIMEZONE_UTC};

/// The built-in default's options: the clock converts with a warning unless `refuse_clock`.
pub fn builtin_conversion_options(refuse_clock: bool) -> ConversionOptions {
    let mut options = ConversionOptions::default();
    if refuse_clock {
        return options;
    }
    let constructs: Vec<String> = BUILTIN_CLOCK_CONSTRUCTS
        .iter()
        .map(|construct| (*construct).to_owned())
        .collect();
    for (divergence, reason, assumption) in [
        (CURRENT_TIME, BUILTIN_CLOCK_REASON, false),
        (SESSION_TIMEZONE_UTC, BUILTIN_CLOCK_ZONE, true),
    ] {
        options.divergences.insert(
            divergence.to_owned(),
            AllowedDivergence {
                constructs: constructs.clone(),
                reason: reason.to_owned(),
                assumption,
            },
        );
    }
    options
}
