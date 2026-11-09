//! `native_predicates`: the expressions starting a predicate, read by a dialect's own parser.

use sqlquack_syntax::Expression;

use crate::dialects::types::StrictParse;
use crate::lint::constants::SELECT_PREFIX;

/// The expressions starting predicate `text`, ignoring what follows, as `native_parse` reads them after SELECT, cut back to where each refusal stopped.
pub(crate) fn native_predicates(
    text: &str,
    native_parse: StrictParse,
) -> Result<Vec<Expression>, String> {
    let mut text: &str = text.trim_end();
    loop {
        let refused: usize = match native_parse(&format!("{SELECT_PREFIX}{text}")) {
            Ok(statements) => {
                return match statements.as_slice() {
                    [Expression::Select(select)] => Ok(select.expressions.clone()),
                    _ => Err("the predicate is not one expression list".to_owned()),
                };
            }
            Err((position, _)) => position,
        };
        let kept: &str = refused
            .checked_sub(SELECT_PREFIX.len())
            .filter(|&at| at > 0 && at < text.len())
            .and_then(|at| text.get(..at))
            .ok_or_else(|| "the predicate does not parse".to_owned())?;
        text = kept.trim_end();
    }
}
