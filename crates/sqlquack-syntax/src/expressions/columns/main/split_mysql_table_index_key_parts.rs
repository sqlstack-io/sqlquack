//! `split_mysql_table_index_key_parts`: splits MySQL table index key parts into plain identifiers or expression-backed parts.

use crate::expressions::names::models::Identifier;
use crate::expressions::tables::models::IndexKeyPart;
use crate::expressions::tree::models::Expression;

/// Splits MySQL table index key parts into plain identifiers or expression-backed parts.
pub fn split_mysql_table_index_key_parts(
    key_parts: Vec<IndexKeyPart>,
) -> (Vec<Identifier>, Vec<IndexKeyPart>) {
    let can_use_legacy_columns = key_parts.iter().all(|part| {
        matches!(part.expression.as_ref(), Expression::Identifier(_))
            && part.prefix_length.is_none()
            && !part.desc
            && !part.asc
    });

    if can_use_legacy_columns {
        let columns: Vec<Identifier> = key_parts
            .into_iter()
            .filter_map(|part| match *part.expression {
                Expression::Identifier(identifier) => Some(identifier),
                _ => None,
            })
            .collect();
        (columns, Vec::new())
    } else {
        (Vec::new(), key_parts)
    }
}
