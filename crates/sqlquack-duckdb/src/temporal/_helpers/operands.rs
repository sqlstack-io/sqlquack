//! Evaluating an operand once where repeating it would grow the emitted SQL.

/// `body` over `value`, inline when it is a plain column or literal, else bound once as `name`.
pub(crate) fn once(value: &str, name: &str, body: impl Fn(&str) -> String) -> String {
    let column = value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'"'));
    let literal = value
        .strip_prefix('\'')
        .and_then(|inner| inner.strip_suffix('\''))
        .is_some_and(|inner| !inner.contains('\''));
    match column || literal {
        true => body(value),
        false => format!(
            "list_transform([{value}], lambda {name}: {})[1]",
            body(name)
        ),
    }
}
