//! Digit strings as numbers, as scan.l and gram.y read integer constants.

/// The value of `text` as digits in `radix`; `None` when empty, not digits, or over 32 bits.
pub fn digits_value(text: &str, radix: u32) -> Option<u32> {
    match text.is_empty() {
        true => None,
        false => text.chars().try_fold(0u32, |value, digit| {
            value
                .checked_mul(radix)?
                .checked_add(digit.to_digit(radix)?)
        }),
    }
}
