//! JSON text of VARIANT-held values as Snowflake writes it.

use crate::macros::constants::VARIANT_WRITTEN_V1;
use crate::macros::main::helper_call::helper_call;

/// The JSON `sql` of a VARIANT-held value as Snowflake writes it, through variant_written_v1.
pub(crate) fn held_written_call(sql: &str) -> String {
    helper_call(VARIANT_WRITTEN_V1, &[sql])
}
