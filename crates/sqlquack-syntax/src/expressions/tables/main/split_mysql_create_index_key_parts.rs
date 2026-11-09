//! `split_mysql_create_index_key_parts`: splits MySQL CREATE INDEX columns into plain columns or expression-backed parts.

use crate::expressions::tables::models::IndexColumn;
use crate::expressions::tables::models::IndexKeyPart;
use crate::expressions::tree::models::Expression;

/// Splits MySQL CREATE INDEX columns into plain columns or expression-backed parts.
pub fn split_mysql_create_index_key_parts(
    key_parts: Vec<IndexKeyPart>,
) -> (Vec<IndexColumn>, Vec<IndexKeyPart>) {
    let can_use_legacy_columns = key_parts.iter().all(|part| {
        matches!(part.expression.as_ref(), Expression::Identifier(_))
            && part.prefix_length.is_none()
    });

    if can_use_legacy_columns {
        let columns: Vec<IndexColumn> = key_parts
            .into_iter()
            .filter_map(|part| {
                let Expression::Identifier(column) = *part.expression else {
                    return None;
                };
                Some(IndexColumn {
                    column,
                    desc: part.desc,
                    asc: part.asc,
                    nulls_first: part.nulls_first,
                    opclass: part.opclass,
                })
            })
            .collect();
        (columns, Vec::new())
    } else {
        (Vec::new(), key_parts)
    }
}
