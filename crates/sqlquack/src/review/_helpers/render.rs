//! One case as a sqllogictest-style review record.

use std::collections::BTreeMap;

use sqlquack_compare::comparison::models::RowOrder;
use sqlquack_compare::values::main::canonical_rows::canonical_rows;
use sqlquack_core::parsing::models::SourceLocation;
use sqlquack_core::provenance::models::Provenance;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::results::main::column_type::column_type;
use sqlquack_evidence::results::models::{RecordedOutcome, StoredResult};
use sqlquack_evidence::verify::models::{CaseVerdict, Conversion, Verdict};

use crate::review::_helpers::values::render_value;

/// Everything shown for one case.
pub(crate) struct CaseReview<'a> {
    pub(crate) case: &'a LoadedCase,
    pub(crate) verdict: Option<&'a CaseVerdict>,
    pub(crate) conversion: Conversion,
    pub(crate) stored: Result<Option<StoredResult>, String>,
}

/// The case's header comments, source SQL, conversion and recorded result.
pub(crate) fn render_case(review: &CaseReview<'_>) -> String {
    let case = review.case;
    let mut text = format!("# case {}\n# {}\n", case.reference, case.description);
    text.push_str(&format!(
        "# verdict: {}\n",
        review.verdict.map_or_else(
            || "not judged".to_owned(),
            |verdict| verdict_label(&verdict.verdict)
        )
    ));
    if let Some(provenance) = review
        .verdict
        .and_then(|verdict| verdict.provenance.as_ref())
    {
        text.push_str(&provenance_comments(provenance));
    }
    text.push_str(&format!(
        "\nsource snowflake\n{}{}\n\n",
        case.setup_sql
            .iter()
            .chain(&case.setup)
            .map(|statement| format!("{statement};\n"))
            .collect::<String>(),
        case.source_sql.trim()
    ));
    text.push_str(&match &review.conversion {
        Conversion::Converted {
            sql, setup, raises, ..
        } => format!(
            "converted duckdb\n{}\n{}\n",
            setup
                .iter()
                .map(|statement| format!("{statement};\n"))
                .chain([sql.clone()])
                .collect::<String>(),
            raise_lines(raises)
        ),
        Conversion::Refused { reasons, .. } => {
            let lines: String = reasons
                .iter()
                .map(|reason| format!("# {reason}\n"))
                .collect();
            format!("refused\n{lines}\n")
        }
    });
    text.push_str(&match &review.stored {
        Ok(stored) => recorded(stored.as_ref(), case.order),
        Err(error) => format!("# the recorded result does not load: {error}\n"),
    });
    text
}

/// The scale of a `NUMBER(p,s)` type name.
fn declared_scale(data_type: &str) -> Option<u8> {
    let arguments = data_type.strip_prefix("NUMBER(")?.strip_suffix(')')?;
    let digits = arguments.split_once(',')?.1.trim();
    if digits.is_empty() {
        return None;
    }
    digits.bytes().try_fold(0_u8, |scale, byte| match byte {
        b'0'..=b'9' => scale.checked_mul(10)?.checked_add(byte - b'0'),
        _ => None,
    })
}

fn verdict_label(verdict: &Verdict) -> String {
    match verdict {
        Verdict::Pending => "pending".to_owned(),
        Verdict::Passed => "passed".to_owned(),
        Verdict::Refused { .. } => "refused".to_owned(),
        Verdict::Diverged { divergence, .. } => format!("diverged as expected ({divergence})"),
        Verdict::Typed => "typed".to_owned(),
        Verdict::Unproven { reason } => format!("unproven: {reason}"),
        Verdict::Failed { reason } => format!("FAILED: {reason}"),
    }
}

fn provenance_comments(provenance: &Provenance) -> String {
    let mut text = String::new();
    for (label, items) in [
        ("equivalences", &provenance.equivalences),
        ("rules", &provenance.rules),
    ] {
        if !items.is_empty() {
            let items: Vec<&str> = items.iter().map(String::as_str).collect();
            text.push_str(&format!("# {label}: {}\n", items.join(", ")));
        }
    }
    for (divergence, constructs) in &provenance.divergences {
        let constructs: Vec<&str> = constructs.iter().map(String::as_str).collect();
        text.push_str(&format!(
            "# warning: known divergence {divergence} at {}\n",
            constructs.join(", ")
        ));
    }
    text
}

fn recorded(stored: Option<&StoredResult>, order: RowOrder) -> String {
    let Some(stored) = stored else {
        return "# no recorded result yet\n".to_owned();
    };
    if stored.metadata.outcome == RecordedOutcome::Types {
        return recorded_types(stored);
    }
    let Some(result) = &stored.rows else {
        let error = stored.metadata.error.as_ref();
        return format!(
            "error snowflake errno {} sqlstate {}\n",
            error
                .and_then(|error| error.errno)
                .map_or_else(|| "unknown".to_owned(), |errno| errno.to_string()),
            error
                .and_then(|error| error.sqlstate.clone())
                .unwrap_or_else(|| "unknown".to_owned())
        );
    };
    let types: Vec<&str> = result
        .columns
        .iter()
        .map(|column| column.snowflake_type.as_str())
        .collect();
    let names: Vec<&str> = result
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    let sort = match order {
        RowOrder::Ordered => "nosort",
        RowOrder::Bag => "rowsort",
    };
    let scales: Vec<Option<u8>> = types
        .iter()
        .map(|data_type| declared_scale(data_type))
        .collect();
    let mut rows: Vec<String> = Vec::new();
    match canonical_rows(&result.rows) {
        Ok(decoded) => {
            for row in decoded {
                let mut cells: Vec<String> = Vec::with_capacity(row.len());
                for (value, scale) in row.iter().zip(&scales) {
                    cells.push(render_value(value, *scale));
                }
                rows.push(cells.join("\t"));
            }
        }
        Err(error) => rows.push(format!("# values not decodable: {error}")),
    }
    if order == RowOrder::Bag {
        rows.sort();
    }
    format!(
        "query snowflake {sort}\n# {}\n# {}\n----\n{}\n",
        names.join("\t"),
        types.join("\t"),
        rows.join("\n")
    )
}

/// A typing result: each column's recorded type and nullability, and the row count.
fn recorded_types(stored: &StoredResult) -> String {
    let mut text = format!(
        "types snowflake, {} rows, values not kept\n",
        stored
            .metadata
            .row_count
            .map_or_else(|| "unknown".to_owned(), |count| count.to_string())
    );
    for column in &stored.metadata.columns {
        let nullable = match column.nullable {
            Some(false) => " NOT NULL",
            _ => "",
        };
        text.push_str(&format!(
            "# {} {}{nullable}\n",
            column.name,
            column_type(column)
        ));
    }
    text
}

/// One `# raise <id> at <line>:<column>, …` comment per raise ID of a conversion.
fn raise_lines(raises: &BTreeMap<String, Vec<SourceLocation>>) -> String {
    let mut lines = String::new();
    for (id, locations) in raises {
        let places: Vec<String> = locations
            .iter()
            .map(|location| format!("{}:{}", location.line, location.column))
            .collect();
        lines.push_str(&format!("# raise {id} at {}\n", places.join(", ")));
    }
    lines
}
