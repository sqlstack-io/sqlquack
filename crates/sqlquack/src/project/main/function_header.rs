//! `function_header`: SQLBuild's `FUNCTION ( key value, ... );` header split into properties and the body.

use crate::project::_helpers::header_parts::{header_and_body, properties};

/// The header's properties as lower-case keys and their values, and the body, or why it is not one.
pub fn function_header(text: &str) -> Result<(Vec<(String, String)>, String), String> {
    let (header, body) = header_and_body(text)?;
    Ok((properties(header)?, body))
}
