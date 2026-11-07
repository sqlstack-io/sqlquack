use std::collections::{HashMap, HashSet};

use sqlquack_syntax::tokens::models::Token;
use sqlquack_syntax::{Dialect, Expression};

/// Inputs for one body's quality Rules, borrowed from the native lint request.
#[derive(Debug)]
pub(crate) struct QualityRequest<'a> {
    pub(crate) statements: &'a [Expression],
    pub(crate) tokens: &'a [Token],
    pub(crate) dialect: Dialect,
    pub(crate) enabled: &'a HashSet<String>,
    pub(crate) max_literal_length: usize,
    pub(crate) max_ranking_order_by: usize,
    pub(crate) relation_keys: &'a HashMap<String, Vec<Vec<String>>>,
    pub(crate) externally_referenced_ctes: &'a HashSet<String>,
    pub(crate) fixture_body: bool,
    /// The configuration file remediation text tells the user to edit.
    pub(crate) config_file: &'a str,
    /// The advice a long literal finding gives before its limit setting.
    pub(crate) long_literal_guidance: &'a str,
}
