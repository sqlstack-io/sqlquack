//! The source SQL a case expands to: typed input rows, then the expression or query.

use crate::cases::models::{CaseDefinition, Spelling};
use crate::constants::{INPUT_RELATION, ORDINAL_COLUMN, VALUE_COLUMN};

pub(crate) fn source_sql(case: &CaseDefinition, spelling: Spelling) -> String {
    let rows: Vec<String> = case
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| input_row(case, index + 1, row))
        .collect();
    let input = format!(
        "WITH {INPUT_RELATION} AS (\n  {}\n)\n",
        rows.join("\n  UNION ALL\n  ")
    );
    let written = match spelling {
        Spelling::Source => case.expression.as_ref().or(case.query.as_ref()),
        Spelling::Canonical => case.canonical.as_ref(),
    };
    match (&case.expression, written) {
        (Some(_), Some(expression)) => format!(
            "{input}SELECT {expression} AS {VALUE_COLUMN}\nFROM {INPUT_RELATION}\nORDER BY {ORDINAL_COLUMN}"
        ),
        (None, Some(query)) => match leading_with(query.trim()) {
            Some(rest) => format!("{}, {rest}", input.trim_end()),
            None => format!("{input}{}", query.trim()),
        },
        (_, None) => input,
    }
}

/// A query's text after a leading `WITH`, whose CTEs follow the input fixture's.
fn leading_with(query: &str) -> Option<&str> {
    let (keyword, rest) = query.split_once(char::is_whitespace)?;
    keyword
        .eq_ignore_ascii_case("WITH")
        .then(|| rest.trim_start())
}

fn input_row(case: &CaseDefinition, ordinal: usize, row: &[String]) -> String {
    let cells: Vec<String> = case
        .columns
        .iter()
        .zip(row)
        .map(|(column, literal)| {
            format!("CAST({literal} AS {}) AS {}", column.data_type, column.name)
        })
        .collect();
    if cells.is_empty() {
        return format!("SELECT {ordinal} AS {ORDINAL_COLUMN}");
    }
    format!("SELECT {ordinal} AS {ORDINAL_COLUMN}, {}", cells.join(", "))
}
