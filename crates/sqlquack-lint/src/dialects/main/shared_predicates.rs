//! `shared_predicates`: the expressions starting a predicate, read by the shared parser.

use sqlquack_parse::parse_expressions;
use sqlquack_syntax::tokens::models::Token;
use sqlquack_syntax::{Dialect, Expression};

/// The expressions starting the predicate tokenized as `tokens`, in `dialect`, ignoring what follows.
pub(crate) fn shared_predicates(
    tokens: &[Token],
    dialect: Dialect,
) -> Result<Vec<Expression>, String> {
    parse_expressions(tokens.to_vec(), dialect).map_err(|error| error.to_string())
}
