//! `faithful_keyword_syntax_name`: sQLQuack: whether `upper_name` has a keyword form (see [`FAITHFUL_KEYWORD_SYNTAX_NAMES`]).

use crate::functions::names::constants::FAITHFUL_KEYWORD_SYNTAX_NAMES;

/// SQLQuack: whether `upper_name` has a keyword form (see [`FAITHFUL_KEYWORD_SYNTAX_NAMES`]).
pub fn faithful_keyword_syntax_name(upper_name: &str) -> bool {
    FAITHFUL_KEYWORD_SYNTAX_NAMES.contains(&upper_name)
}
