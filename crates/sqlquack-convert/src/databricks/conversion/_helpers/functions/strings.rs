//! String functions, with Databricks's handling of empty, NULL and out-of-range arguments written out.

use crate::databricks::conversion::_helpers::expressions::literals::{quoted_text, typed};
use crate::databricks::conversion::_helpers::expressions::patterns::case_mapped;
use crate::databricks::conversion::_helpers::functions::arrays::captured;
use crate::databricks::conversion::_helpers::functions::rewrites::{Rewrite, small_integer};
use crate::databricks::conversion::constants::{
    ALPHABET_ERROR, ASCII_ALPHABET, ASCII_LOWER, ASCII_UPPER, CASE_MAPPING_ERROR,
    CHARACTER_KIND_ERROR, CHR_MODULUS, DEFAULT_MASKS, DEFAULT_PAD, KINDED_CHARACTERS,
    LAMBDA_ELEMENT, LAMBDA_INDEX, LOWERED_CHARACTERS, MASKED_KINDS, SOUNDEX_CODES,
    SUBSTRING_LENGTH, UNTITLED_CHARACTERS, UPPERED_CHARACTERS,
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
        "LOWER" | "LCASE" => |values| case_function(values, "lower", LOWERED_CHARACTERS),
        "UPPER" | "UCASE" => |values| case_function(values, "upper", UPPERED_CHARACTERS),
        "INITCAP" => initcap,
        "MASK" => mask,
        "LUHN_CHECK" => luhn_check,
        "LEVENSHTEIN" => levenshtein,
        "SOUNDEX" => soundex,
        "IS_VALID_UTF8" => |values| valid_text(values, true),
        "VALIDATE_UTF8" | "TRY_VALIDATE_UTF8" | "MAKE_VALID_UTF8" => {
            |values| valid_text(values, false)
        }
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

/// A test of two strings, or of byte strings and the UTF-8 bytes of strings compared as hex at whole bytes; TRUE for an empty second argument.
fn tested(values: &[Emitted], function: &str) -> Option<Emitted> {
    let [text, part] = values else { return None };
    let sql = match (text.data_type, part.data_type) {
        (DatabricksType::String, DatabricksType::String) => {
            format!("{function}({}, {})", text.sql, part.sql)
        }
        (
            DatabricksType::String | DatabricksType::Binary,
            DatabricksType::String | DatabricksType::Binary,
        ) => {
            let (a, b) = (format!("hex({})", text.sql), format!("hex({})", part.sql));
            match function {
                "contains" => format!("regexp_matches({a}, '^(..)*' || {b})"),
                _ => format!("{function}({a}, {b})"),
            }
        }
        _ => return None,
    };
    Some(typed(sql, DatabricksType::Boolean))
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
pub(crate) fn strings<const N: usize>(values: &[Emitted]) -> Option<[&Emitted; N]> {
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

/// `INITCAP(s)`: `s` lowered, then its first character and each after a space in title case, raising where Java's title case is not DuckDB's `upper`.
fn initcap(values: &[Emitted]) -> Option<Emitted> {
    let [text] = strings::<1>(values)?;
    let (t, e) = (&text.sql, LAMBDA_ELEMENT);
    let lowered = case_mapped(t, "lower", LOWERED_CHARACTERS);
    Some(typed(
        format!(
            "(CASE WHEN regexp_matches(lower({t}), '(^| )[{UNTITLED_CHARACTERS}]') THEN error('{CASE_MAPPING_ERROR}') \
             ELSE array_to_string(list_transform(string_split({lowered}, ' '), \
             lambda {e}: upper(left({e}, 1)) || substr({e}, 2)), ' ') END)"
        ),
        DatabricksType::String,
    ))
}

/// `MASK(s[, upper[, lower[, digit[, other]]]])`: each character as the one-character mask of its kind, raising for one whose kind RE2 may read otherwise.
fn mask(values: &[Emitted]) -> Option<Emitted> {
    let (text, given) = values.split_first()?;
    if text.data_type != DatabricksType::String || given.len() > DEFAULT_MASKS.len() {
        return None;
    }
    let mut masks: Vec<Option<String>> = DEFAULT_MASKS
        .iter()
        .map(|mask| mask.map(quoted_text))
        .collect();
    for (slot, value) in masks.iter_mut().zip(given) {
        match &value.literal {
            Some(Literal::Text(mask)) if mask.chars().count() == 1 => {
                *slot = Some(quoted_text(mask))
            }
            _ => return None,
        }
    }
    let e = LAMBDA_ELEMENT;
    let mut kinds = String::new();
    for (kind, mask) in MASKED_KINDS.iter().zip(&masks) {
        if let Some(mask) = mask {
            kinds.push_str(&format!("WHEN regexp_matches({e}, '{kind}') THEN {mask} "));
        }
    }
    let other = masks
        .last()
        .cloned()
        .flatten()
        .unwrap_or_else(|| e.to_owned());
    let t = &text.sql;
    Some(typed(
        format!(
            "(CASE WHEN regexp_matches({t}, '[^{KINDED_CHARACTERS}]') THEN error('{CHARACTER_KIND_ERROR}') \
             WHEN {t} = '' THEN '' ELSE array_to_string(list_transform(string_split({t}, ''), \
             lambda {e}: CASE {kinds}ELSE {other} END), '') END)"
        ),
        DatabricksType::String,
    ))
}

/// `LUHN_CHECK(s)`: whether `s` is ASCII digits whose Luhn sum ends in 0, false for other text; raising for another script's digit, which no rule reads.
fn luhn_check(values: &[Emitted]) -> Option<Emitted> {
    let [text] = strings::<1>(values)?;
    let (t, c, i) = (&text.sql, LAMBDA_ELEMENT, LAMBDA_INDEX);
    Some(typed(
        format!(
            "(CASE WHEN {t} IS NULL THEN NULL \
             WHEN regexp_matches(regexp_replace({t}, '[0-9]', '', 'g'), '\\p{{Nd}}') THEN error('{CHARACTER_KIND_ERROR}') \
             WHEN NOT regexp_full_match({t}, '[0-9]+') THEN FALSE \
             ELSE list_sum(list_transform(string_split(reverse({t}), ''), lambda {c}, {i}: \
             CASE WHEN {i} % 2 = 0 THEN (CAST({c} AS INTEGER) * 2) % 10 + (CAST({c} AS INTEGER) * 2) // 10 \
             ELSE CAST({c} AS INTEGER) END)) % 10 = 0 END)"
        ),
        DatabricksType::Boolean,
    ))
}

/// `LEVENSHTEIN(a, b)`: DuckDB's byte distance once each distinct character is one ASCII byte, raising for more than ASCII holds.
fn levenshtein(values: &[Emitted]) -> Option<Emitted> {
    let [left, right] = strings::<2>(values)?;
    let (a, b, e) = (&left.sql, &right.sql, LAMBDA_ELEMENT);
    let characters = format!("list_sort(list_distinct(string_split({a} || {b}, '')))");
    let encoded = format!(
        "translate({a} || {b}, array_to_string({characters}, ''), \
         array_to_string(list_transform(range(1, {}), lambda {e}: chr(CAST({e} AS INTEGER))), ''))",
        ASCII_ALPHABET + 1
    );
    Some(typed(
        format!(
            "(CASE WHEN {a} IS NULL OR {b} IS NULL THEN NULL \
             WHEN len({characters}) > {ASCII_ALPHABET} THEN CAST(error('{ALPHABET_ERROR}') AS INTEGER) \
             ELSE CAST(levenshtein(left({encoded}, length({a})), substr({encoded}, length({a}) + 1)) AS INTEGER) END)"
        ),
        DatabricksType::Int,
    ))
}

/// `IS_VALID_UTF8(s)` with `test`, else `VALIDATE_UTF8`, `TRY_VALIDATE_UTF8` or `MAKE_VALID_UTF8`: DuckDB holds only valid UTF-8 in a string, so each converted string is valid and kept.
fn valid_text(values: &[Emitted], test: bool) -> Option<Emitted> {
    let [text] = strings::<1>(values)?;
    Some(match test {
        true => typed(
            format!("({} IS NOT NULL OR NULL)", text.sql),
            DatabricksType::Boolean,
        ),
        false => typed(text.sql.clone(), DatabricksType::String),
    })
}

/// `SOUNDEX(s)` of text starting with an ASCII letter, read byte by byte with H and W skipped after the first, and other text as it is.
fn soundex(values: &[Emitted]) -> Option<Emitted> {
    let [text] = strings::<1>(values)?;
    if captured(&text.sql) {
        return None;
    }
    let (t, e, i) = (&text.sql, LAMBDA_ELEMENT, LAMBDA_INDEX);
    let mapped = format!(
        "translate(regexp_replace(translate({t}, '{ASCII_LOWER}', '{ASCII_UPPER}'), '[^A-Z]', '0', 'g'), \
         '{ASCII_UPPER}', '{SOUNDEX_CODES}')"
    );
    let codes = format!(
        "string_split(translate(left({mapped}, 1), '7', 'x') || replace(substr({mapped}, 2), '7', ''), '')"
    );
    Some(typed(
        format!(
            "(CASE WHEN {t} IS NULL THEN NULL WHEN NOT regexp_matches({t}, '^[A-Za-z]') THEN {t} \
             ELSE upper(left({t}, 1)) || rpad(left(replace(array_to_string(\
             list_filter({codes}, lambda {e}, {i}: {i} = 1 OR {e} <> {codes}[{i} - 1])[2:], ''), '0', ''), 3), 3, '0') END)"
        ),
        DatabricksType::String,
    ))
}
