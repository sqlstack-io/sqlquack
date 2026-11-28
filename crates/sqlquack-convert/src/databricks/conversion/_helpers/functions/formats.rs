//! DATE_FORMAT and TO_CHAR of a date or timestamp with a constant pattern whose Java letters `strftime` writes alike.

use crate::databricks::conversion::_helpers::expressions::literals::{
    cast_value, quoted_text, typed,
};
use crate::databricks::conversion::_helpers::functions::rewrites::Rewrite;
use crate::databricks::conversion::constants::{DATE_PATTERN_LETTERS, UNFORMATTED_YEAR_ERROR};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal};

/// The rewrite of the formatting function `name`, `None` when none handles it.
pub(crate) fn format_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "DATE_FORMAT" => |values| date_format(values, true),
        "TO_CHAR" | "TO_VARCHAR" => |values| date_format(values, false),
        _ => return None,
    };
    Some(rewrite)
}

/// `DATE_FORMAT(x, pattern)` of a date, timestamp or, with `text`, text: `x` written by the pattern, raising past year 9999.
fn date_format(values: &[Emitted], text: bool) -> Option<Emitted> {
    let [value, pattern] = values else {
        return None;
    };
    let Some(Literal::Text(pattern)) = &pattern.literal else {
        return None;
    };
    let value = match value.data_type {
        DatabricksType::Date | DatabricksType::Timestamp | DatabricksType::TimestampNtz => {
            value.clone()
        }
        DatabricksType::String if text => cast_value(value, DatabricksType::Timestamp)?,
        _ => return None,
    };
    let format = strftime_format(pattern)?;
    Some(typed(
        format!(
            "(CASE WHEN {x} IS NULL THEN NULL WHEN year({x}) BETWEEN 1 AND 9999 THEN strftime({x}, {}) \
             ELSE CAST(error({}) AS VARCHAR) END)",
            quoted_text(&format),
            quoted_text(UNFORMATTED_YEAR_ERROR),
            x = value.sql
        ),
        DatabricksType::String,
    ))
}

/// The Java pattern `pattern` as a `strftime` format, `None` for a letter, count or symbol outside those it writes alike.
fn strftime_format(pattern: &str) -> Option<String> {
    let characters: Vec<char> = pattern.chars().collect();
    let mut format = String::new();
    let mut at = 0;
    while let Some(&character) = characters.get(at) {
        if character == '\'' {
            at += 1;
            if characters.get(at) == Some(&'\'') {
                format.push('\'');
                at += 1;
                continue;
            }
            loop {
                match characters.get(at)? {
                    '\'' if characters.get(at + 1) == Some(&'\'') => {
                        format.push('\'');
                        at += 2;
                    }
                    '\'' => break,
                    '%' => {
                        format.push_str("%%");
                        at += 1;
                    }
                    &quoted => {
                        format.push(quoted);
                        at += 1;
                    }
                }
            }
            at += 1;
            continue;
        }
        let run = characters[at..]
            .iter()
            .take_while(|&&next| next == character)
            .count();
        at += run;
        if character.is_ascii_alphabetic() {
            let letters = character.to_string().repeat(run);
            let (_, written) = DATE_PATTERN_LETTERS
                .iter()
                .find(|(java, _)| *java == letters)?;
            format.push_str(written);
            continue;
        }
        if matches!(character, '[' | ']' | '{' | '}' | '#') {
            return None;
        }
        for _ in 0..run {
            match character {
                '%' => format.push_str("%%"),
                character => format.push(character),
            }
        }
    }
    Some(format)
}
