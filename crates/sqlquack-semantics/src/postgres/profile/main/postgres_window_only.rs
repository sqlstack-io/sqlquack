//! `postgres_window_only`: whether a call names a function PostgreSQL's catalogue runs only with OVER.

use sqlquack_syntax::Expression;

use crate::postgres::profile::_helpers::typing::{called, window_function};

/// Whether `expression` calls a built-in window function, which PostgreSQL runs only with OVER.
pub(crate) fn postgres_window_only(expression: &Expression) -> bool {
    called(expression).is_some_and(|(name, _)| window_function(&name))
}
