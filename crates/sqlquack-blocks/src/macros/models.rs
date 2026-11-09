//! One helper macro.

/// A helper macro: its versioned name, parameters, body and what `sqlquack explain` says of it.
#[derive(Clone, Copy, Debug)]
pub struct HelperMacro {
    /// The name in the `sqlquack` schema, ending in its version, such as `variant_text_v1`.
    pub name: &'static str,
    /// The parameter names, in order; a raising macro takes its raise site first, as `site`.
    pub parameters: &'static [&'static str],
    /// The body; a version's body never changes once released.
    pub body: fn() -> String,
    /// What it computes, for `sqlquack explain`.
    pub explain: &'static str,
}
