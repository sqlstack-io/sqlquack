//! String functions, with Databricks's handling of empty, NULL and out-of-range arguments written out.

use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::expressions::patterns::case_mapped;
use crate::databricks::conversion::_helpers::functions::rewrites::{Rewrite, small_integer};
use crate::databricks::conversion::constants::{
    CHR_MODULUS, DEFAULT_PAD, LOWERED_CHARACTERS, PATTERN_ERROR, REGEX_METACHARACTER_CLASS,
    REGEX_METACHARACTERS, SUBSTRING_LENGTH, UPPERED_CHARACTERS,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal};

/// The rewrite of the string function `name`, `None` when none handles it; lengths count code points in both engines.
pub(crate) fn string_rewrite(name: &str) -> Option<Rewrite> {
    let rewrite: Rewrite = match name {
        "LENGTH" | "LEN" | "CHAR_LENGTH" | "CHARACTER_LENGTH" => {
            |values| sized(values, "length({s})", 1)
        }
        "OCTET_LENGTH" => |values| sized(values, "strlen({s})", 1),
        "BIT_LENGTH" => |values| sized(values, "strlen({s})", 8),
        "MD5" => |values| digest(values, "md5"),
        "SHA" | "SHA1" => |values| digest(values, "sha1"),
        "ASCII" => |values| counted(values, "ascii({s})"),
        "INSTR" => |values| found(values, false),
        "LOCATE" | "POSITION" | "CHARINDEX" => |values| found(values, true),
        "CONTAINS" => |values| tested(values, "contains"),
        "STARTSWITH" => |values| tested(values, "starts_with"),
        "ENDSWITH" => |values| tested(values, "ends_with"),
        "RIGHT" => right,
        "SUBSTR" | "SUBSTRING" => substring,
        "OVERLAY" => overlay,
        "LPAD" => |values| padded(values, "lpad"),
        "RPAD" => |values| padded(values, "rpad"),
        "SPACE" => space,
        "CHR" | "CHAR" => chr,
        "REPLACE" => replace,
        "TRANSLATE" => translate,
        "BTRIM" => btrim,
        "CONCAT_WS" => concat_ws,
        "FIND_IN_SET" => find_in_set,
        "RLIKE" | "REGEXP" | "REGEXP_LIKE" => literal_pattern,
        "LOWER" | "LCASE" => |values| case_function(values, "lower", LOWERED_CHARACTERS),
        "UPPER" | "UCASE" => |values| case_function(values, "upper", UPPERED_CHARACTERS),
        _ => return None,
    };
    Some(rewrite)
}

/// A count over one string as INT.
fn counted(values: &[Emitted], template: &str) -> Option<Emitted> {
    let [text] = strings::<1>(values)?;
    Some(typed(
        format!("CAST({} AS INTEGER)", template.replace("{s}", &text.sql)),
        DatabricksType::Int,
    ))
}

/// A length of a string as INT, or of a byte string its bytes times `bits`, as Databricks counts both.
fn sized(values: &[Emitted], template: &str, bits: u8) -> Option<Emitted> {
    let [value] = values else { return None };
    let count = match value.data_type {
        DatabricksType::String => template.replace("{s}", &value.sql),
        DatabricksType::Binary => format!("octet_length({})", value.sql),
        _ => return None,
    };
    let scaled = match bits {
        1 => count,
        bits => format!("{count} * {bits}"),
    };
    Some(typed(
        format!("CAST({scaled} AS INTEGER)"),
        DatabricksType::Int,
    ))
}

/// A digest of a string's UTF-8 bytes or of a byte string, in lower-case hex as both engines write it.
pub(crate) fn digest(values: &[Emitted], function: &str) -> Option<Emitted> {
    let [value] = values else { return None };
    matches!(
        value.data_type,
        DatabricksType::String | DatabricksType::Binary
    )
    .then(|| typed(format!("{function}({})", value.sql), DatabricksType::String))
}

/// `INSTR(s, t)`, or with `swapped` `LOCATE(t, s)`: the 1-based character position of `t` in `s`, 0 if absent.
fn found(values: &[Emitted], swapped: bool) -> Option<Emitted> {
    let [first, second] = strings::<2>(values)?;
    let (haystack, needle) = match swapped {
        true => (second, first),
        false => (first, second),
    };
    Some(typed(
        format!("CAST(instr({}, {}) AS INTEGER)", haystack.sql, needle.sql),
        DatabricksType::Int,
    ))
}

/// A string test of two strings, TRUE for an empty second string.
fn tested(values: &[Emitted], function: &str) -> Option<Emitted> {
    let [text, part] = strings::<2>(values)?;
    Some(typed(
        format!("{function}({}, {})", text.sql, part.sql),
        DatabricksType::Boolean,
    ))
}

/// `RIGHT(s, n)`: '' for a zero or negative length, where DuckDB counts from the start.
fn right(values: &[Emitted]) -> Option<Emitted> {
    let [text, length] = values else { return None };
    if text.data_type != DatabricksType::String || !small_integer(length.data_type) {
        return None;
    }
    Some(typed(
        format!(
            "CASE WHEN {s} IS NULL OR {n} IS NULL THEN NULL WHEN {n} <= 0 THEN '' ELSE right({s}, {n}) END",
            s = text.sql,
            n = length.sql
        ),
        DatabricksType::String,
    ))
}

/// `SUBSTR(s, pos[, len])` as Spark's substringSQL: a negative `pos` counts from the end, 0 is 1, `len` defaults to INT's largest.
fn substring(values: &[Emitted]) -> Option<Emitted> {
    let (text, position, length) = match values {
        [text, position] => (text, position, SUBSTRING_LENGTH.to_owned()),
        [text, position, length] if small_integer(length.data_type) => {
            (text, position, format!("CAST({} AS BIGINT)", length.sql))
        }
        _ => return None,
    };
    if text.data_type != DatabricksType::String || !small_integer(position.data_type) {
        return None;
    }
    let (s, p) = (&text.sql, &position.sql);
    Some(typed(
        format!(
            "CASE WHEN {s} IS NULL OR {p} IS NULL OR {length} IS NULL THEN NULL ELSE {} END",
            substring_sql(s, p, &length)
        ),
        DatabricksType::String,
    ))
}

/// Spark's substringSQL of non-NULL `s` from `p` for `length` characters, as BIGINT arithmetic.
fn substring_sql(s: &str, p: &str, length: &str) -> String {
    let start = format!(
        "(CASE WHEN {p} > 0 THEN CAST({p} AS BIGINT) - 1 WHEN {p} < 0 THEN length({s}) + {p} ELSE 0 END)"
    );
    let end = format!("({start} + {length})");
    let from = format!("greatest({start}, 0)");
    format!(
        "(CASE WHEN {from} >= {end} THEN '' ELSE substring({s}, {from} + 1, {end} - {from}) END)"
    )
}

/// `OVERLAY(s PLACING r FROM p [FOR n])` as Databricks computes it: `p - 1` and `p + n` never wrap.
fn overlay(values: &[Emitted]) -> Option<Emitted> {
    let (text, replacement, position, length) = match values {
        [text, replacement, position] => (text, replacement, position, None),
        [text, replacement, position, length] => (text, replacement, position, Some(length)),
        _ => return None,
    };
    let strings = [text, replacement]
        .iter()
        .all(|value| value.data_type == DatabricksType::String);
    let counts = small_integer(position.data_type)
        && length.is_none_or(|length| small_integer(length.data_type));
    if !strings || !counts {
        return None;
    }
    let (s, r, p) = (&text.sql, &replacement.sql, &position.sql);
    let skipped = match length {
        Some(length) => format!(
            "CASE WHEN {n} >= 0 THEN CAST({n} AS BIGINT) ELSE length({r}) END",
            n = length.sql
        ),
        None => format!("length({r})"),
    };
    let before = format!("(CAST({p} AS BIGINT) - 1)");
    let after = format!("(CAST({p} AS BIGINT) + {skipped})");
    let nulls = std::iter::once(&text.sql)
        .chain([r, p])
        .chain(length.map(|length| &length.sql))
        .map(|sql| format!("{sql} IS NULL"))
        .collect::<Vec<_>>()
        .join(" OR ");
    Some(typed(
        format!(
            "CASE WHEN {nulls} THEN NULL ELSE {} || {r} || {} END",
            substring_sql(s, "1", &before),
            substring_sql(s, &after, SUBSTRING_LENGTH)
        ),
        DatabricksType::String,
    ))
}

/// `LPAD` or `RPAD(s, n[, pad])`: '' for `n` of zero or less, `s` cut to `n` for an empty pad.
fn padded(values: &[Emitted], function: &str) -> Option<Emitted> {
    let (text, length, pad) = match values {
        [text, length] => (text, length, DEFAULT_PAD.to_owned()),
        [text, length, pad] if pad.data_type == DatabricksType::String => {
            (text, length, pad.sql.clone())
        }
        _ => return None,
    };
    if text.data_type != DatabricksType::String || !small_integer(length.data_type) {
        return None;
    }
    Some(typed(
        format!(
            "CASE WHEN {s} IS NULL OR {n} IS NULL OR {pad} IS NULL THEN NULL WHEN {n} <= 0 THEN '' \
             WHEN {pad} = '' THEN left({s}, {n}) ELSE {function}({s}, {n}, {pad}) END",
            s = text.sql,
            n = length.sql
        ),
        DatabricksType::String,
    ))
}

/// `SPACE(n)`: `n` spaces, '' for zero or less, as `repeat` gives.
fn space(values: &[Emitted]) -> Option<Emitted> {
    let [count] = values else { return None };
    small_integer(count.data_type).then(|| {
        typed(
            format!("repeat({DEFAULT_PAD}, {})", count.sql),
            DatabricksType::String,
        )
    })
}

/// `CHR(n)`: '' for a negative `n`, else the character of `n` modulo 256.
fn chr(values: &[Emitted]) -> Option<Emitted> {
    let [code] = values else { return None };
    code.data_type.is_integral().then(|| {
        typed(
            format!(
                "CASE WHEN {n} < 0 THEN '' ELSE chr(CAST({n} % {CHR_MODULUS} AS INTEGER)) END",
                n = code.sql
            ),
            DatabricksType::String,
        )
    })
}

/// `REPLACE(s, search[, by])`: every `search` replaced, `s` unchanged for an empty `search`.
fn replace(values: &[Emitted]) -> Option<Emitted> {
    let (text, search, by) = match values {
        [text, search] => (text, search, "''".to_owned()),
        [text, search, by] if by.data_type == DatabricksType::String => {
            (text, search, by.sql.clone())
        }
        _ => return None,
    };
    strings::<2>(&[text.clone(), search.clone()])?;
    Some(typed(
        format!("replace({}, {}, {by})", text.sql, search.sql),
        DatabricksType::String,
    ))
}

/// `TRANSLATE(s, from, to)`: the first mapping of a repeated character wins in both engines.
fn translate(values: &[Emitted]) -> Option<Emitted> {
    let [text, from, to] = strings::<3>(values)?;
    Some(typed(
        format!("translate({}, {}, {})", text.sql, from.sql, to.sql),
        DatabricksType::String,
    ))
}

/// `BTRIM(s)`: spaces trimmed from both ends.
fn btrim(values: &[Emitted]) -> Option<Emitted> {
    let [text] = strings::<1>(values)?;
    Some(typed(
        format!("trim({}, {DEFAULT_PAD})", text.sql),
        DatabricksType::String,
    ))
}

/// `CONCAT_WS(sep, s, ...)` of strings: NULLs skipped, '' with no strings, NULL for a NULL `sep`.
fn concat_ws(values: &[Emitted]) -> Option<Emitted> {
    let text = |value: &Emitted| {
        matches!(
            value.data_type,
            DatabricksType::String | DatabricksType::Null
        )
    };
    if values.is_empty() || !values.iter().all(text) {
        return None;
    }
    let separator = &values[0].sql;
    let sql = match values {
        [_] => format!("CASE WHEN {separator} IS NULL THEN NULL ELSE '' END"),
        _ => {
            let parts: Vec<&str> = values.iter().map(|value| value.sql.as_str()).collect();
            format!("concat_ws({})", parts.join(", "))
        }
    };
    Some(typed(sql, DatabricksType::String))
}

/// `FIND_IN_SET(s, list)`: the 1-based place of `s` among `list`'s comma-separated items, 0 if absent or `s` has a comma.
fn find_in_set(values: &[Emitted]) -> Option<Emitted> {
    let [item, items] = strings::<2>(values)?;
    Some(typed(
        format!(
            "CASE WHEN {s} IS NULL OR {list} IS NULL THEN NULL WHEN contains({s}, ',') THEN 0 \
             ELSE CAST(coalesce(list_position(string_split({list}, ','), {s}), 0) AS INTEGER) END",
            s = item.sql,
            list = items.sql
        ),
        DatabricksType::Int,
    ))
}

/// Exactly `N` arguments, all strings.
fn strings<const N: usize>(values: &[Emitted]) -> Option<[&Emitted; N]> {
    let strings = values.len() == N
        && values
            .iter()
            .all(|value| value.data_type == DatabricksType::String);
    strings.then(|| std::array::from_fn(|index| &values[index]))
}

/// `LOWER` or `UPPER` of a string, raising for a character whose mapping DuckDB was not seen to share with Java.
fn case_function(values: &[Emitted], function: &str, characters: &str) -> Option<Emitted> {
    let [text] = values else { return None };
    (text.data_type == DatabricksType::String).then(|| {
        typed(
            case_mapped(&text.sql, function, characters),
            DatabricksType::String,
        )
    })
}

/// `RLIKE(s, p)` of a pattern without a regular-expression metacharacter, which finds `p` as text; a column pattern raises for one.
fn literal_pattern(values: &[Emitted]) -> Option<Emitted> {
    let [text, pattern] = strings::<2>(values)?;
    let found = format!("contains({}, {})", text.sql, pattern.sql);
    let sql = match &pattern.literal {
        Some(Literal::Text(literal))
            if literal.contains(|character| REGEX_METACHARACTERS.contains(character)) =>
        {
            return None;
        }
        Some(_) => found,
        None => format!(
            "(CASE WHEN regexp_matches({}, '[{REGEX_METACHARACTER_CLASS}]') THEN error('{PATTERN_ERROR}') \
             ELSE {found} END)",
            pattern.sql
        ),
    };
    Some(typed(sql, DatabricksType::Boolean))
}
