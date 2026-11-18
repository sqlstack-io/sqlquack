//! `checked_call_depth`: a function-call limit a caller asked for, within what the parser allows.

use crate::parsing::constants::MAX_CALL_DEPTH_CEILING;

/// `depth` when it is between 1 and the ceiling, or why it is refused, naming the ceiling.
pub fn checked_call_depth(depth: usize) -> Result<usize, String> {
    match depth {
        1..=MAX_CALL_DEPTH_CEILING => Ok(depth),
        _ => Err(format!(
            "the function-call depth limit must be between 1 and {MAX_CALL_DEPTH_CEILING}, \
             SQLQuack's fixed nesting ceiling, not {depth}"
        )),
    }
}
