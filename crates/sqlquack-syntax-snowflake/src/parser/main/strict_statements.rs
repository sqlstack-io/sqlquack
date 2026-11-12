//! `strict_statements`: the statements of `sql`, refusing on any parse error or uncovered byte.

use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::guard::models::ParseOptions;
use sqlquack_syntax::lossless::main::lossless_tokens::lossless_tokens;
use sqlquack_syntax::tokens::models::{CallMode, Dialect};

use crate::modes::errors::StrictFailure;
use crate::parser::_helpers::support::dialects::dialect_type;

/// The statements of `sql`, coverage checked over the tokens read; inlined so `parse_strict` holds it.
#[inline(always)]
pub(crate) fn strict_statements(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>, StrictFailure> {
    let parser = crate::parser::models::Dialect::get(dialect_type(dialect)?);
    let (statements, significant) =
        parser.parse_reading_tokens(sql, options, CallMode::Faithful)?;
    lossless_tokens(sql, &significant, parser.tokenizer.nested_comments())?;
    Ok(statements)
}
