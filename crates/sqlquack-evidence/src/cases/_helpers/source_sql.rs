//! The source SQL a case expands to: typed input rows, then the expression or query.

use crate::cases::models::{CaseDefinition, Spelling};
use crate::constants::{INPUT_RELATION, ORDINAL_COLUMN, ROW_COUPLING_WORDS, VALUE_COLUMN};

pub(crate) fn source_sql(case: &CaseDefinition, spelling: Spelling) -> String {
    numbered_source_sql(case, spelling, 1)
}

/// The source SQL with the input rows numbered from `first`, so a row alone keeps its ordinal.
fn numbered_source_sql(case: &CaseDefinition, spelling: Spelling, first: usize) -> String {
    let rows: Vec<String> = case
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| input_row(case, index + first, row))
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

/// Each input row's source alone, for a case whose result rows follow its input rows one for one.
pub(crate) fn row_sources(case: &CaseDefinition, spelling: Spelling) -> Vec<String> {
    let independent = match (&case.expression, &case.query) {
        (Some(expression), _) => !coupled(expression),
        (None, Some(query)) => row_by_row(query),
        (None, None) => false,
    };
    match independent && case.rows.len() > 1 {
        true => case
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                numbered_source_sql(
                    &CaseDefinition {
                        rows: vec![row.clone()],
                        ..case.clone()
                    },
                    spelling,
                    index + 1,
                )
            })
            .collect(),
        false => Vec::new(),
    }
}

/// Whether `query` reads `input` in order with no clause or call that couples rows.
fn row_by_row(query: &str) -> bool {
    let upper = query.to_ascii_uppercase();
    upper.contains(&format!("FROM {INPUT_RELATION} ORDER BY {ORDINAL_COLUMN}").to_ascii_uppercase())
        && upper.matches(" FROM ").count() == 1
        && !coupled(query)
}

/// Whether `sql` holds a clause or call that makes one result row depend on other input rows.
fn coupled(sql: &str) -> bool {
    let upper = sql.to_ascii_uppercase();
    ROW_COUPLING_WORDS.iter().any(|word| upper.contains(word))
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
