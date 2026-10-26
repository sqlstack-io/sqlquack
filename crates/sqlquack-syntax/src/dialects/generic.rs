//! Generic SQL Dialect

use super::DialectImpl;

/// Generic SQL dialect (ANSI SQL)
pub(crate) struct GenericDialect;

impl DialectImpl for GenericDialect {}
