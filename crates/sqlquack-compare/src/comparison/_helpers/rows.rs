//! Row-by-row value comparison.

use crate::comparison::models::{Mismatch, MismatchKind, RowOrder};
use crate::values::main::canonical_rows::canonical_rows;
use crate::values::models::{CanonicalValue, ResultSet};

pub(crate) struct RowRules {
    pub(crate) order: RowOrder,
    pub(crate) float32_ulps: u32,
    pub(crate) float64_ulps: u32,
    pub(crate) ignore_zone_labels: bool,
    /// Columns holding VARIANT text, compared without the whitespace outside JSON strings.
    pub(crate) json_columns: Vec<bool>,
}

pub(crate) fn compare_rows(
    expected: &ResultSet,
    actual: &ResultSet,
    rules: &RowRules,
) -> Result<(), Mismatch> {
    let mut expected_rows = decoded(expected, rules)?;
    let mut actual_rows = decoded(actual, rules)?;
    if expected_rows.len() != actual_rows.len() {
        return Err(Mismatch {
            kind: MismatchKind::RowCount,
            detail: format!(
                "recorded {} rows, compared result has {}",
                expected_rows.len(),
                actual_rows.len()
            ),
        });
    }
    if rules.order == RowOrder::Bag {
        expected_rows.sort();
        actual_rows.sort();
    }
    for (row, (expected_row, actual_row)) in expected_rows.iter().zip(&actual_rows).enumerate() {
        for (column, (expected_value, actual_value)) in
            expected_row.iter().zip(actual_row).enumerate()
        {
            if !values_match(expected_value, actual_value, rules) {
                return Err(Mismatch {
                    kind: MismatchKind::Value,
                    detail: format!(
                        "row {row}, column {column}: recorded {expected_value:?}, compared {actual_value:?}"
                    ),
                });
            }
        }
    }
    Ok(())
}

fn decoded(result: &ResultSet, rules: &RowRules) -> Result<Vec<Vec<CanonicalValue>>, Mismatch> {
    let rows = canonical_rows(result).map_err(|detail| Mismatch {
        kind: MismatchKind::Unsupported,
        detail,
    })?;
    let mut normalised_rows: Vec<Vec<CanonicalValue>> = Vec::with_capacity(rows.len());
    for row in rows {
        normalised_rows.push(normalised_row(row, rules));
    }
    Ok(normalised_rows)
}

fn normalised_row(row: Vec<CanonicalValue>, rules: &RowRules) -> Vec<CanonicalValue> {
    row.into_iter()
        .enumerate()
        .map(|(index, value)| normalised(value, index, rules))
        .collect()
}

fn normalised(value: CanonicalValue, index: usize, rules: &RowRules) -> CanonicalValue {
    match value {
        CanonicalValue::Text(text) if rules.json_columns.get(index).copied().unwrap_or(false) => {
            CanonicalValue::Text(compact_json(&text))
        }
        timestamp if rules.ignore_zone_labels => without_zone_label(timestamp),
        other => other,
    }
}

/// JSON text without the whitespace outside strings.
fn compact_json(text: &str) -> String {
    let mut compact = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for character in text.chars() {
        match (in_string, escaped, character) {
            (true, true, _) => escaped = false,
            (true, false, '\\') => escaped = true,
            (true, false, '"') | (false, _, '"') => in_string = !in_string,
            (false, _, ' ' | '\n' | '\r' | '\t') => continue,
            _ => {}
        }
        compact.push(character);
    }
    compact
}

fn without_zone_label(value: CanonicalValue) -> CanonicalValue {
    match value {
        CanonicalValue::Timestamp {
            nanos,
            zone: Some(_),
        } => CanonicalValue::Timestamp {
            nanos,
            zone: Some(String::new()),
        },
        other => other,
    }
}

fn values_match(expected: &CanonicalValue, actual: &CanonicalValue, rules: &RowRules) -> bool {
    match (expected, actual) {
        (CanonicalValue::Float32(left), CanonicalValue::Float32(right))
            if rules.float32_ulps > 0 =>
        {
            ulps_apart(i64::from(ordered32(*left)), i64::from(ordered32(*right)))
                <= u64::from(rules.float32_ulps)
        }
        (CanonicalValue::Float64(left), CanonicalValue::Float64(right))
            if rules.float64_ulps > 0 =>
        {
            ulps_apart(ordered64(*left), ordered64(*right)) <= u64::from(rules.float64_ulps)
        }
        _ => expected == actual,
    }
}

fn ordered32(bits: u32) -> i32 {
    let signed = i32::from_ne_bytes(bits.to_ne_bytes());
    if signed < 0 {
        i32::MIN - signed
    } else {
        signed
    }
}

fn ordered64(bits: u64) -> i64 {
    let signed = i64::from_ne_bytes(bits.to_ne_bytes());
    if signed < 0 {
        i64::MIN - signed
    } else {
        signed
    }
}

fn ulps_apart(left: i64, right: i64) -> u64 {
    left.abs_diff(right)
}
