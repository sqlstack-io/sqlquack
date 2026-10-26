//! Where an expression is converted: its scope, the statement context and its clause.

use crate::conversion::_helpers::support::scope::{Context, Scope};

/// Whether aggregates and windows may appear where an expression sits, and if it is a WHERE filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Placement {
    pub(crate) aggregates: bool,
    pub(crate) windows: bool,
    pub(crate) filter: bool,
}

impl Placement {
    pub(crate) const ROW: Self = Self {
        aggregates: false,
        windows: false,
        filter: false,
    };
    pub(crate) const FILTER: Self = Self {
        aggregates: false,
        windows: false,
        filter: true,
    };
    pub(crate) const PROJECTION: Self = Self {
        aggregates: true,
        windows: true,
        filter: false,
    };
    pub(crate) const GROUPED: Self = Self {
        aggregates: true,
        windows: false,
        filter: false,
    };
}

/// Everything an expression rule needs besides the node.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Site<'a> {
    pub(crate) scope: &'a Scope<'a>,
    pub(crate) context: &'a Context<'a>,
    pub(crate) placement: Placement,
}

impl<'a> Site<'a> {
    /// The same site in another clause.
    pub(crate) fn placed(self, placement: Placement) -> Self {
        Self { placement, ..self }
    }

    /// The same site in another scope.
    pub(crate) fn scoped(self, scope: &'a Scope<'a>) -> Self {
        Self { scope, ..self }
    }
}
