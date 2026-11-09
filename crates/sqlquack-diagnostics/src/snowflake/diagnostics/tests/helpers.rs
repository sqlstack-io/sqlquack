use sqlquack_syntax::Dialect;

use crate::classification::models::{Construct, ConstructFamily};
use crate::diagnostics::main::{diagnose::diagnose, render_diagnostics::render_diagnostics};
use crate::refusals::main::refuse_unparsed::refuse_unparsed;
use crate::refusals::models::{Refusal, RefusalCategory};
use sqlquack_parse::parsing::main::parse_source::parse_source;
use sqlquack_syntax::tokens::models::{SourceLines, SourceLocation};

/// A refusal of `category` at characters `start..end` of `source`.
pub(crate) fn placed(
    source: &str,
    (start, end): (usize, usize),
    category: RefusalCategory,
) -> Refusal {
    Refusal {
        construct: Some(Construct::new(ConstructFamily::Function, "EDITDISTANCE")),
        location: Some(SourceLines::new(source).location(start, end)),
        category,
        reason: "no rule converts this call".to_owned(),
    }
}

/// The refusal the parser's failure on `source` gives.
pub(crate) fn unparsed(source: &str) -> Refusal {
    refuse_unparsed(
        &parse_source(source, Dialect::Snowflake).expect_err("test SQL does not parse"),
        &SourceLines::new(source),
    )
}

/// The rendered text of one refusal of `source`.
pub(crate) fn rendered(source: &str, refusal: Refusal) -> String {
    render_diagnostics(&diagnose(&[refusal], source), source, "query.sql")
}

/// A refusal with no place, such as one of the whole input.
pub(crate) fn unlocated(location: Option<SourceLocation>) -> Refusal {
    Refusal {
        construct: None,
        location,
        category: RefusalCategory::ConversionBudget,
        reason: "conversion took over 5 s".to_owned(),
    }
}
