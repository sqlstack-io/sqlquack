//! `parse_strict`: the statements of `sql`, refusing on any parse error or any byte no token covers.

use crate::modes::errors::StrictFailure;
use crate::parser::_helpers::support::dialects::dialect_type;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::guard::models::ParseOptions;
use sqlquack_syntax::lossless::main::lossless_tokens::lossless_tokens;
use sqlquack_syntax::tokens::models::{CallMode, Dialect, Span, TokenType};

/// The statements of `sql`, refusing on any parse error or any byte no token covers; the
/// coverage check reads the tokens the parser read rather than tokenizing `sql` again.
pub fn parse_strict(
    sql: &str,
    dialect: Dialect,
    options: &ParseOptions,
) -> Result<Vec<Expression>, StrictFailure> {
    let parser = crate::parser::models::Dialect::get(dialect_type(dialect)?);
    let mut significant: Vec<(Span, TokenType)> = Vec::new();
    let statements =
        parser.parse_reading_tokens(sql, options, CallMode::Faithful, &mut significant)?;
    lossless_tokens(sql, &significant, parser.tokenizer.nested_comments())?;
    Ok(statements)
}
