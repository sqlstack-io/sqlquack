//! Keyword lists of SqlBaseParser.g4.

/// `strictNonReserved`: the only words a table alias (`strictIdentifier`) cannot be.
pub const STRICT_NON_RESERVED: [&str; 16] = [
    "ANTI",
    "CROSS",
    "EXCEPT",
    "FULL",
    "INNER",
    "INTERSECT",
    "JOIN",
    "LATERAL",
    "LEFT",
    "MINUS",
    "NATURAL",
    "ON",
    "RIGHT",
    "SEMI",
    "UNION",
    "USING",
];
