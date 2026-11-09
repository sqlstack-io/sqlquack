//! `canonical_rows`: every row of a result set as canonical values.

use crate::values::_helpers::decode::column_values;
use crate::values::models::{CanonicalValue, ResultSet};

/// The rows of `result`, or the first column whose Arrow type has no canonical form.
pub fn canonical_rows(result: &ResultSet) -> Result<Vec<Vec<CanonicalValue>>, String> {
    let mut rows: Vec<Vec<CanonicalValue>> = Vec::new();
    for batch in &result.batches {
        let columns = batch
            .columns()
            .iter()
            .map(|column| column_values(column.as_ref()))
            .collect::<Result<Vec<_>, String>>()?;
        for row in 0..batch.num_rows() {
            rows.push(columns.iter().map(|column| column[row].clone()).collect());
        }
    }
    Ok(rows)
}
