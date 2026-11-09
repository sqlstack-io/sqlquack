//! `format_sql`: SQL laid out over lines, with exactly its tokens apart from keyword case.

use crate::format::_helpers::pipeline::format_checked;
use crate::format::models::{FormatOptions, FormatRefusal, Formatted};

/// `sql` laid out over lines; refused, with a reason, when that cannot be done safely.
pub fn format_sql(sql: &str, options: &FormatOptions) -> Result<Formatted, FormatRefusal> {
    format_checked(sql, options).map_err(|reason| FormatRefusal { reason })
}
