//! Where an expression is converted: its scope, the statement context and its clause.

use crate::conversion::_helpers::support::scope::{Context, Scope};

/// Whether aggregates and windows may appear where an expression sits, and if it is a WHERE filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Placement {
    pub(crate) aggregates: bool,
    pub(crate) windows: bool,
    pub(crate) filter: bool,
    /// Whether an unordered array may pass up unchecked, as only order-free readers take it.
    pub(crate) order_free: bool,
    /// Whether suffixed SYSTEM$TYPEOF text may pass up, as only readers keeping it whole take it.
    pub(crate) suffixed: bool,
}

impl Placement {
    pub(crate) const ROW: Self = Self {
        aggregates: false,
        windows: false,
        filter: false,
        order_free: false,
        suffixed: false,
    };
    pub(crate) const FILTER: Self = Self {
        aggregates: false,
        windows: false,
        filter: true,
        order_free: false,
        suffixed: false,
    };
    pub(crate) const PROJECTION: Self = Self {
        aggregates: true,
        windows: true,
        filter: false,
        order_free: false,
        suffixed: false,
    };
    pub(crate) const GROUPED: Self = Self {
        aggregates: true,
        windows: false,
        filter: false,
        order_free: false,
        suffixed: false,
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

    /// The same site where an unordered array may pass up unchecked, or with `free` false not.
    pub(crate) fn order_free(self, free: bool) -> Self {
        Self {
            placement: Placement {
                order_free: free,
                ..self.placement
            },
            ..self
        }
    }

    /// The same site where SYSTEM$TYPEOF text with its suffix on may pass up, or with `free` false not.
    pub(crate) fn suffix_free(self, free: bool) -> Self {
        Self {
            placement: Placement {
                suffixed: free,
                ..self.placement
            },
            ..self
        }
    }

    /// The same site in another scope.
    pub(crate) fn scoped(self, scope: &'a Scope<'a>) -> Self {
        Self { scope, ..self }
    }
}
