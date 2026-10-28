//! How text is read as a TIME.

/// How a text is read as a TIME.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeFormat {
    /// Without a format: `H:M[:S[.fraction]]`.
    Auto,
    /// `HH24:MI`.
    HourMinute,
    /// `HH24MI`.
    CompactHourMinute,
}
