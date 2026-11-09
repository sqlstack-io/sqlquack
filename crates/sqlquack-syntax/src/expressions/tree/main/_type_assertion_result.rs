//! `type_assertion_result`: the type of an expression with a type assertion, which fills only unknown inferred types.

use crate::expressions::types::models::DataType;

/// The type of an expression with a type assertion, which fills only unknown inferred types.
pub(in crate::expressions) fn type_assertion_result<'a>(
    inferred: Option<&'a DataType>,
    asserted: &'a DataType,
) -> &'a DataType {
    match inferred {
        Some(inferred) if inferred.is_refined_by(asserted) => asserted,
        Some(inferred) => inferred,
        None => asserted,
    }
}
