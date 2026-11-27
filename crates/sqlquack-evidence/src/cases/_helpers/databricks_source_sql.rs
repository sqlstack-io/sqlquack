//! The source SQL of a case recorded on Databricks SQL: the input rows as one `VALUES` list.

use crate::cases::models::{CaseDefinition, Spelling};
use crate::constants::{INPUT_RELATION, ORDINAL_COLUMN, VALUE_COLUMN};

/// `WITH input AS (SELECT * FROM VALUES (1, CAST(v AS T)), ... AS input(ordinal, ...)) <query>`.
pub(crate) fn databricks_source_sql(case: &CaseDefinition, spelling: Spelling) -> String {
    let written = match spelling {
        Spelling::Source => case.expression.as_ref().or(case.query.as_ref()),
        Spelling::Canonical => case.canonical.as_ref(),
    };
    let body: String = match (&case.expression, written) {
        (Some(_), Some(expression)) => format!(
            "SELECT {expression} AS {VALUE_COLUMN} FROM {INPUT_RELATION} ORDER BY {ORDINAL_COLUMN}"
        ),
        (None, Some(query)) => query.clone(),
        (_, None) => String::new(),
    };
    if case.rows.is_empty() {
        return body;
    }
    let names: Vec<&str> = std::iter::once(ORDINAL_COLUMN)
        .chain(case.columns.iter().map(|column| column.name.as_str()))
        .collect();
    let rows: Vec<String> =
        case.rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let cells: Vec<String> =
                    std::iter::once((index + 1).to_string())
                        .chain(row.iter().zip(&case.columns).map(|(value, column)| {
                            format!("CAST({value} AS {})", column.data_type)
                        }))
                        .collect();
                format!("({})", cells.join(", "))
            })
            .collect();
    format!(
        "WITH {INPUT_RELATION} AS (SELECT * FROM VALUES {} AS {INPUT_RELATION}({})) {body}",
        rows.join(", "),
        names.join(", ")
    )
}
