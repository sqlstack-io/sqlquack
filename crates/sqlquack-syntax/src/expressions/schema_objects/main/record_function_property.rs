//! `record_function_property`.

use crate::expressions::schema_objects::models::CreateFunction;
use crate::expressions::schema_objects::models::FunctionPropertyKind;

pub(crate) fn record_function_property(
    mut function: CreateFunction,
    kind: FunctionPropertyKind,
) -> CreateFunction {
    if !function.property_order.contains(&kind) {
        function.property_order.push(kind);
    }
    function
}
