//! `parse_dollar_string_token`: splits a DollarString token's text at '\x00' into tag and content.

/// Splits a DollarString token's text at '\x00' into tag and content.
pub fn parse_dollar_string_token(text: &str) -> (Option<String>, String) {
    if let Some(pos) = text.find('\x00') {
        let tag = &text[..pos];
        let content = &text[pos + 1..];
        (Some(tag.to_string()), content.to_string())
    } else {
        (None, text.to_string())
    }
}
