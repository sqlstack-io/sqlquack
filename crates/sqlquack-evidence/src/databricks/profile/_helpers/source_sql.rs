//! The source SQL of a case recorded on Databricks SQL: the input rows as one `VALUES` list.

use crate::cases::models::{CaseDefinition, Spelling};
use crate::constants::{INPUT_RELATION, ORDINAL_COLUMN, VALUE_COLUMN};

/// `WITH input AS (SELECT * FROM VALUES (1, CAST(v AS T)), ... AS input(ordinal, ...)) <query>`.
pub(crate) fn values_source_sql(case: &CaseDefinition, spelling: Spelling) -> String {
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
    let mut names: Vec<&str> = vec![ORDINAL_COLUMN];
    names.extend(case.columns.iter().map(|column| column.name.as_str()));
    let mut rows: Vec<String> = Vec::with_capacity(case.rows.len());
    for (index, row) in case.rows.iter().enumerate() {
        let mut cells: Vec<String> = vec![(index + 1).to_string()];
        for (value, column) in row.iter().zip(&case.columns) {
            cells.push(format!("CAST({value} AS {})", column.data_type));
        }
        rows.push(format!("({})", cells.join(", ")));
    }
    format!(
        "WITH {INPUT_RELATION} AS (SELECT * FROM VALUES {} AS {INPUT_RELATION}({})) {body}",
        rows.join(", "),
        names.join(", ")
    )
}
