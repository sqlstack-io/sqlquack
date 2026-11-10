//! Session constants.

use sqlquack_policy::policy::constants::{DECLARED_OFFSET, SESSION_TIMEZONE_UTC};

/// The known differences strict mode still allows, as facts: a UTC session and declared offsets.
pub const STRICT_FACTS: [&str; 2] = [SESSION_TIMEZONE_UTC, DECLARED_OFFSET];
