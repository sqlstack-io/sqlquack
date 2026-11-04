//! `timestamp_format`: a literal Snowflake timestamp format as the pattern it reads.

use crate::temporal::constants::{
    LITERAL_FORMAT_LETTERS, LITERAL_FORMAT_PUNCTUATION, TIMESTAMP_FORMAT_ELEMENTS,
};
use crate::temporal::models::{TimestampField, TimestampFormat};

/// What a four-digit year reads, in place of YY in the pattern of recorded NULLs.
const FOUR_DIGIT_YEAR: &str = "([0-9]{4})";

/// The whole-text pattern of a format of recorded elements, or what the recordings do not cover.
pub fn timestamp_format(written: &str) -> Result<TimestampFormat, String> {
    let mut pattern = String::from("^");
    let mut fields: Vec<TimestampField> = Vec::new();
    let mut rest = written;
    let mut short_year: Option<(usize, usize)> = None;
    let mut previous_numeric = false;
    while let Some(next) = rest.chars().next() {
        if let Some((name, field, reads)) = TIMESTAMP_FORMAT_ELEMENTS.iter().find(|(name, _, _)| {
            rest.get(..name.len())
                .is_some_and(|start| start.eq_ignore_ascii_case(name))
        }) {
            if fields.contains(field) {
                return Err(format!("the format element {name} twice"));
            }
            rest = &rest[name.len()..];
            let numeric = !matches!(field, TimestampField::Meridiem | TimestampField::MonthName);
            let adjacent = numeric
                && (previous_numeric
                    || TIMESTAMP_FORMAT_ELEMENTS.iter().any(|(other, kind, _)| {
                        !matches!(kind, TimestampField::Meridiem | TimestampField::MonthName)
                            && rest
                                .get(..other.len())
                                .is_some_and(|start| start.eq_ignore_ascii_case(other))
                    }));
            fields.push(*field);
            let reads = match adjacent {
                true => reads.replace("{1,2}", "{2}"),
                false => (*reads).to_owned(),
            };
            if *field == TimestampField::ShortYear {
                short_year = Some((pattern.len(), reads.len()));
            }
            pattern.push_str(&reads);
            previous_numeric = numeric;
        } else if next == '"' {
            let Some(end) = rest[1..].find('"') else {
                return Err("an unclosed quoted literal".to_owned());
            };
            pattern.push_str(&literal(&rest[1..=end]));
            rest = &rest[end + 2..];
            previous_numeric = false;
        } else if next == ' ' {
            pattern.push_str(" *");
            rest = rest.trim_start_matches(' ');
            previous_numeric = false;
        } else if LITERAL_FORMAT_PUNCTUATION.contains(next) || LITERAL_FORMAT_LETTERS.contains(next)
        {
            pattern.push_str(&literal(&rest[..1]));
            rest = &rest[1..];
            previous_numeric = false;
        } else {
            return Err(format!(
                "the format text '{next}', which no recording shows"
            ));
        }
    }
    pattern.push('$');
    let has = |field: TimestampField| fields.contains(&field);
    let complete = (has(TimestampField::Year) != has(TimestampField::ShortYear))
        && (has(TimestampField::Month) != has(TimestampField::MonthName))
        && has(TimestampField::Day);
    let clock = !(has(TimestampField::Hour24) && has(TimestampField::Hour12))
        && has(TimestampField::Hour12) == has(TimestampField::Meridiem)
        && (has(TimestampField::Minute) || !has(TimestampField::Second))
        && (has(TimestampField::Hour24)
            || has(TimestampField::Hour12)
            || !has(TimestampField::Minute));
    match complete && clock {
        true => Ok(TimestampFormat {
            rejected: short_year.map(|(start, length)| {
                format!(
                    "{}{FOUR_DIGIT_YEAR}{}",
                    &pattern[..start],
                    &pattern[start + length..]
                )
            }),
            pattern,
            fields,
        }),
        false => Err("a format without a year, month and day and a whole clock".to_owned()),
    }
}

/// `text` as a pattern matching it literally.
fn literal(text: &str) -> String {
    let mut pattern = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() || character == ' ' {
            pattern.push(character);
        } else if character == '\'' {
            pattern.push_str("''");
        } else {
            pattern.push('\\');
            pattern.push(character);
        }
    }
    pattern
}
