//! Canonical values and result sets.

use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;

/// One cell in a physical-type-independent form; equal values are structurally equal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalValue {
    /// SQL NULL.
    Null,
    /// A boolean.
    Boolean(bool),
    /// An exact number with trailing zeros of the fraction removed.
    Number {
        /// The digits as an integer.
        unscaled: i128,
        /// Digits after the decimal point.
        scale: i8,
    },
    /// A 32-bit float's bits; every NaN has one representation.
    Float32(u32),
    /// A 64-bit float's bits; every NaN has one representation.
    Float64(u64),
    /// Text.
    Text(String),
    /// Bytes.
    Binary(Vec<u8>),
    /// Days since 1970-01-01.
    Date(i64),
    /// Nanoseconds since midnight.
    Time(i64),
    /// Nanoseconds since 1970-01-01 00:00, and the zone label of zoned timestamps.
    Timestamp {
        /// Nanoseconds since the epoch, in UTC for zoned timestamps.
        nanos: i128,
        /// The Arrow zone label; absent for naive timestamps.
        zone: Option<String>,
    },
}

/// A query result: its Arrow schema and batches.
#[derive(Clone, Debug)]
pub struct ResultSet {
    /// The schema, present even without rows.
    pub schema: SchemaRef,
    /// The rows.
    pub batches: Vec<RecordBatch>,
}
