//! Raises found in emitted SQL.

/// One raise's tag and ID marker in emitted SQL, as byte ranges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RaiseSlot {
    /// The tag: `sqlquack:<errno>` or a divergence name.
    pub(crate) tag: std::ops::Range<usize>,
    /// `#` and the ID, or `#?` while unidentified.
    pub(crate) marker: std::ops::Range<usize>,
    /// Whether the raise still waits for its ID.
    pub(crate) unidentified: bool,
}
