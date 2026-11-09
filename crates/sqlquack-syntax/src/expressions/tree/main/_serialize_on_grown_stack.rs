//! `serialize_on_grown_stack`: serializes a recursive field on a fresh stack segment when the stack runs low.

use serde::Serialize;
use serde::Serializer;

use crate::expressions::tree::main::_on_grown_stack::on_grown_stack;

/// Serializes `value`, moving to a fresh stack segment first when the current stack is nearly used.
pub(in crate::expressions) fn serialize_on_grown_stack<T, S>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    T: Serialize + ?Sized,
    S: Serializer,
{
    on_grown_stack(|| value.serialize(serializer))
}
