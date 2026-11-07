//! `builtin_conversion_options`: the built-in default's options: the clock and reads of it.

use crate::conversion::models::{AllowedDivergence, ConversionOptions};
use crate::session::constants::{
    BUILTIN_CLOCK_CONSTRUCTS, BUILTIN_CLOCK_REASON, BUILTIN_CLOCK_ZONE,
    BUILTIN_SUBMICRO_CONSTRUCTS, BUILTIN_SUBMICRO_REASON, BUILTIN_ZONE_CONSTRUCTS,
};
use sqlquack_policy::policy::constants::{
    CURRENT_TIME, SESSION_TIMEZONE_UTC, TIMESTAMP_TZ_SUBMICRO,
};

/// The built-in default's options: the clock converts with a warning unless `refuse_clock`.
pub fn builtin_conversion_options(refuse_clock: bool) -> ConversionOptions {
    let mut options = ConversionOptions::default();
    if refuse_clock {
        return options;
    }
    for (divergence, constructs, reason, assumption) in [
        (
            CURRENT_TIME,
            BUILTIN_CLOCK_CONSTRUCTS.as_slice(),
            BUILTIN_CLOCK_REASON,
            false,
        ),
        (
            SESSION_TIMEZONE_UTC,
            BUILTIN_ZONE_CONSTRUCTS.as_slice(),
            BUILTIN_CLOCK_ZONE,
            true,
        ),
        (
            TIMESTAMP_TZ_SUBMICRO,
            BUILTIN_SUBMICRO_CONSTRUCTS.as_slice(),
            BUILTIN_SUBMICRO_REASON,
            false,
        ),
    ] {
        options.divergences.insert(
            divergence.to_owned(),
            AllowedDivergence {
                constructs: constructs
                    .iter()
                    .map(|construct| (*construct).to_owned())
                    .collect(),
                reason: reason.to_owned(),
                assumption,
            },
        );
    }
    options
}
