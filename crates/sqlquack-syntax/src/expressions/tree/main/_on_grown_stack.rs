//! `on_grown_stack`: runs a step of a recursive walk on a fresh stack segment when the stack runs low.

/// Runs `operation`, moving to a fresh stack segment first when the current stack is nearly used.
pub(in crate::expressions) fn on_grown_stack<R>(operation: impl FnOnce() -> R) -> R {
    #[cfg(feature = "stacker")]
    {
        stacker::maybe_grow(256 * 1024, 4 * 1024 * 1024, operation)
    }

    #[cfg(not(feature = "stacker"))]
    {
        operation()
    }
}
