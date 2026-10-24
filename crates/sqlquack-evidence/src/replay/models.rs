//! Replay settings.

/// The DuckDB session a case replays in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplaySettings {
    /// DuckDB's `TimeZone`.
    pub time_zone: String,
    /// DuckDB's `default_null_order`.
    pub null_order: String,
}
