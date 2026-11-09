//! Whether SQL that did not parse holds dbt Jinja, which only `dbt compile` turns into SQL.

use crate::diagnostics::constants::JINJA_MARKERS;
use crate::refusals::models::Refusal;

/// Whether the line the parser stopped on, or the whole source without a location, holds Jinja.
pub(crate) fn holds_jinja(refusal: &Refusal, source: &str) -> bool {
    let text = refusal
        .location
        .and_then(|location| source.lines().nth(location.line.saturating_sub(1)))
        .unwrap_or(source);
    JINJA_MARKERS.iter().any(|marker| text.contains(marker))
}
