//! Scopes and what a name resolves to in one.

use crate::types::models::ColumnSpec;

/// A relation visible in a FROM clause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeRelation {
    /// The name the relation is read by.
    pub alias: String,
    /// Its columns, in order.
    pub columns: Vec<ColumnSpec>,
}

/// What a query level can see, select-list aliases bound to values of type `A`.
#[derive(Clone, Debug)]
pub struct Scope<'a, A> {
    /// The relations of its FROM clause.
    pub relations: Vec<ScopeRelation>,
    /// Select-list and lambda aliases, the latest last.
    pub aliases: Vec<(String, A)>,
    /// The enclosing query's scope, for correlated names.
    pub outer: Option<&'a Scope<'a, A>>,
    /// Whether unqualified names resolve to select aliases first, as in Snowflake's ORDER BY.
    pub aliases_first: bool,
}

impl<'a, A> Scope<'a, A> {
    /// A scope over `relations` with no aliases, inside `outer`.
    pub fn new(relations: Vec<ScopeRelation>, outer: Option<&'a Scope<'a, A>>) -> Self {
        Self {
            relations,
            aliases: Vec::new(),
            outer,
            aliases_first: false,
        }
    }
}

/// What a name resolves to: a relation's column, or the value an alias is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution<'s, A> {
    /// A column of the relation read by `alias`.
    Column {
        /// The relation's name in scope.
        alias: &'s str,
        /// The column.
        column: &'s ColumnSpec,
    },
    /// A select-list or lambda alias.
    Alias(&'s A),
}

impl<'s, A> Resolution<'s, A> {
    /// The relation's name in scope and the column, when the name is a column.
    pub fn column(&self) -> Option<(&'s str, &'s ColumnSpec)> {
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
