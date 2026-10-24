//! The source SQL a case expands to: typed input rows, then the expression or query.

use crate::cases::models::CaseDefinition;
use crate::constants::{INPUT_RELATION, ORDINAL_COLUMN, VALUE_COLUMN};

pub(crate) fn source_sql(case: &CaseDefinition) -> String {
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
    match (&case.expression, &case.query) {
        (Some(expression), _) => format!(
            "{input}SELECT {expression} AS {VALUE_COLUMN}\nFROM {INPUT_RELATION}\nORDER BY {ORDINAL_COLUMN}"
        ),
        (None, Some(query)) => format!("{input}{}", query.trim()),
        (None, None) => input,
    }
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
    format!("SELECT {ordinal} AS {ORDINAL_COLUMN}, {}", cells.join(", "))
}
