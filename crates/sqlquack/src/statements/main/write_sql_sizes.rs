//! `write_sql_sizes`: the bytes of DuckDB SQL each converted case emits, as a JSON file.

use std::collections::BTreeMap;
use std::path::Path;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde_json::json;
use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::verify::models::Conversion;
use sqlquack_evidence::verify::types::CaseConverter;

use crate::analysis::models::PipelineConverter;
use crate::statements::constants::SQL_SIZES_FORMAT;

/// Writes each converted case's SQL size in bytes to `path`, and how many, or why it could not.
pub fn write_sql_sizes(
    path: &Path,
    cases: &[LoadedCase],
    converter: &PipelineConverter,
) -> Result<usize, String> {
    let sizes: BTreeMap<&str, usize> = cases
        .par_iter()
        .filter_map(|case| match converter.convert(case) {
            Conversion::Converted { sql, .. } => Some((case.reference.as_str(), sql.len())),
            Conversion::Refused { .. } => None,
        })
        .collect();
    let count = sizes.len();
    let file = json!({ "format": SQL_SIZES_FORMAT, "cases": sizes });
    let text = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())? + "\n";
    std::fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(count)
}
