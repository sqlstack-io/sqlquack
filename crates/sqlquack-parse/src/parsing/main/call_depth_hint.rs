//! `call_depth_hint`: what a refusal for nesting past a limit tells the caller.

use crate::parsing::constants::{
    AST_DEPTH_CODE, CALL_DEPTH_CEILING_NOTE, FUNCTION_NESTING_CODE, MAX_CALL_DEPTH_CEILING,
    NESTING_CODE,
};

/// `message` naming the fixed ceiling it met, or how to raise `limit` through `setting`.
pub fn call_depth_hint(message: String, limit: usize, setting: Option<&str>) -> String {
    let function_nesting: bool = message.contains(FUNCTION_NESTING_CODE);
    if message.contains(AST_DEPTH_CODE) && limit >= MAX_CALL_DEPTH_CEILING {
        return format!("{message}. {CALL_DEPTH_CEILING_NOTE}");
    }
    if message.contains(NESTING_CODE) || (function_nesting && limit >= MAX_CALL_DEPTH_CEILING) {
        return format!(
            "{message}; {MAX_CALL_DEPTH_CEILING} is SQLQuack's fixed nesting ceiling, which no \
             setting raises"
        );
    }
    match (function_nesting, setting) {
        (true, Some(setting)) => {
            format!("{message}; raise {setting}, up to {MAX_CALL_DEPTH_CEILING}")
        }
        _ => message,
    }
}
