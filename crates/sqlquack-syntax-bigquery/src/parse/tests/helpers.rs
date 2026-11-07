use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::ExpectedVerdict;

/// The verdict strict parsing gives `sql`: accepted, opaque when it kept a gap, or rejected.
pub(crate) fn verdict(sql: &str) -> ExpectedVerdict {
    parse_strict(sql)
        .map(|parsed| ExpectedVerdict::Accepted {
            opaque: !parsed.gaps.is_empty(),
        })
        .unwrap_or_else(|failure| ExpectedVerdict::Rejected {
            syntax: failure.parse_error().is_none_or(|error| error.syntax),
        })
}
