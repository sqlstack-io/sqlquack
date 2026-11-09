//! Scopes and what a name resolves to in one.

use crate::schemas::models::ColumnSpec;
use crate::schemas::types::DialectType;

/// A relation visible in a FROM clause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeRelation<T: DialectType> {
    /// The name the relation is read by.
    pub alias: String,
    /// Its columns, in order.
    pub columns: Vec<ColumnSpec<T>>,
    /// Columns a JOIN USING merged into an earlier relation's: read only qualified, and not by a bare star.
    pub merged: Vec<String>,
}

impl<T: DialectType> Default for ScopeRelation<T> {
    fn default() -> Self {
        Self {
            alias: String::new(),
            columns: Vec::new(),
            merged: Vec::new(),
        }
    }
}

/// What a query level can see, select-list aliases bound to values of type `A`.
#[derive(Clone, Debug)]
pub struct Scope<'a, T: DialectType, A> {
    /// The relations of its FROM clause.
    pub relations: Vec<ScopeRelation<T>>,
    /// Select-list and lambda aliases, the latest last.
    pub aliases: Vec<(String, A)>,
    /// The enclosing query's scope, for correlated names.
    pub outer: Option<&'a Scope<'a, T, A>>,
    /// Whether unqualified names resolve to select aliases first, as an ORDER BY may.
    pub aliases_first: bool,
    /// Whether the query level has a GROUP BY, so its aggregates never see an empty input.
    pub grouped: bool,
}

impl<'a, T: DialectType, A> Scope<'a, T, A> {
    /// A scope over `relations` with no aliases, inside `outer`.
    pub fn new(relations: Vec<ScopeRelation<T>>, outer: Option<&'a Scope<'a, T, A>>) -> Self {
        Self {
            relations,
            aliases: Vec::new(),
            outer,
            aliases_first: false,
            grouped: false,
        }
    }
}

/// What a name resolves to: a relation's column, or the value an alias is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution<'s, T: DialectType, A> {
    /// A column of the relation read by `alias`.
    Column {
        /// The relation's name in scope.
        alias: &'s str,
        /// The column.
        column: &'s ColumnSpec<T>,
    },
    /// A select-list or lambda alias.
    Alias(&'s A),
}

impl<'s, T: DialectType, A> Resolution<'s, T, A> {
    /// The relation's name in scope and the column, when the name is a column.
    pub fn column(&self) -> Option<(&'s str, &'s ColumnSpec<T>)> {
        match *self {
            Self::Column { alias, column } => Some((alias, column)),
            Self::Alias(_) => None,
        }
    }

    /// The bound value, when the name is an alias.
    pub fn alias_value(&self) -> Option<&'s A> {
        match *self {
            Self::Alias(value) => Some(value),
            Self::Column { .. } => None,
        }
    }
}
