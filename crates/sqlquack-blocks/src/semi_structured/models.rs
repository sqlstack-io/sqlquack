//! The steps of a VARIANT path, the keys OBJECT_PICK keeps, and recursive FLATTEN modes.

/// One step of a path: an object key or an array index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathStep {
    /// A key, matched with regard to case.
    Key(String),
    /// A zero-based array index.
    Index(u64),
}

/// The keys OBJECT_PICK keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickedKeys {
    /// Literal keys.
    Literal(Vec<String>),
    /// The SQL of a VARIANT array of strings.
    Array(String),
}

/// A typed SQL value put in a VARIANT, by how DuckDB holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypedSource {
    /// A DATE.
    Date,
    /// A TIMESTAMP_NTZ of up to six digits, held as TIMESTAMP.
    Timestamp,
    /// A TIMESTAMP_NTZ of seven to nine digits, held as TIMESTAMP_NS.
    TimestampNanos,
    /// A TIMESTAMP_LTZ, held as TIMESTAMPTZ.
    TimestampLocal,
}

/// Which containers a recursive FLATTEN descends into, as its MODE argument names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlattenMode {
    /// Objects and arrays.
    Both,
    /// Objects only.
    Object,
    /// Arrays only.
    Array,
}
