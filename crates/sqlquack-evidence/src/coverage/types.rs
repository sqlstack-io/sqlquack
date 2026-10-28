//! Coverage type aliases.

use std::collections::{BTreeMap, BTreeSet};

use sqlquack_core::provenance::models::Shape;

use crate::coverage::models::Covering;

/// Each rule's passing shapes.
pub(crate) type RuleCoverings<'a> = BTreeMap<&'a str, Vec<Covering<'a>>>;

/// Each rule's converted shapes, passing or not.
pub(crate) type RuleShapes<'a> = BTreeMap<&'a str, BTreeSet<&'a Shape>>;
