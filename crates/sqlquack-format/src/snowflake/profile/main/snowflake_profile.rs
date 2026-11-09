//! `snowflake_profile`: how Snowflake SQL is read and laid out.

use sqlquack_syntax::Dialect;

use crate::dialects::models::{FormatProfile, LayoutRules};
use crate::snowflake::profile::constants::INLINE_CALLS;

/// How Snowflake SQL is read and laid out.
pub(crate) fn snowflake_profile() -> FormatProfile {
    FormatProfile {
        dialect: Dialect::Snowflake,
        lex: None,
        keywords: None,
        parse: None,
        rules: LayoutRules {
            inline_calls: &INLINE_CALLS,
            refuses_binds: true,
            keeps_unaliased_items: true,
            breaks_conditions: true,
            path_colons: true,
            stage_runs: true,
            ..LayoutRules::default()
        },
    }
}
