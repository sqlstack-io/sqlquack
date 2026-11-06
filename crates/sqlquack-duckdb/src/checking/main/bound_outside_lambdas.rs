//! `bound_outside_lambdas`: a value DuckDB cannot bind inside a lambda, bound once outside them.

use crate::checking::_helpers::lambda_bodies::{Body, bodies_around, lambda_bodies, reads_word};
use crate::checking::constants::LAMBDA_KEYWORD;

/// `sql` with `value` bound outside the lambdas holding it, if any; or the lambda parameter it reads.
pub fn bound_outside_lambdas(sql: &str, value: &str, parameter: &str) -> Result<Option<String>, String> {
    let bodies = lambda_bodies(sql);
    let starts: Vec<usize> = sql.match_indices(value).map(|(at, _)| at).collect();
    let enclosing: Vec<&Body> = starts
        .iter()
        .flat_map(|at| bodies_around(&bodies, *at))
        .collect();
    if enclosing.is_empty() {
        return Ok(None);
    }
    let own: Vec<String> = lambda_bodies(value)
        .into_iter()
        .flat_map(|body| body.parameters)
        .collect();
    let read = enclosing
        .iter()
        .flat_map(|body| &body.parameters)
        .find(|name| !own.contains(name) && reads_word(value, name));
    match read {
        Some(name) => Err(name.clone()),
        None => Ok(Some(format!(
            "list_transform([{value}], {LAMBDA_KEYWORD}{parameter}: {})[1]",
            sql.replace(value, parameter)
        ))),
    }
}
