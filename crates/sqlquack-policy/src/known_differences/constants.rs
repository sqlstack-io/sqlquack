//! Every known difference, described in plain English.

use crate::known_differences::models::KnownDifference;
use crate::policy::models::DivergenceFailure;

/// One entry per `[divergences.<name>]` of the repository's `sqlquack.toml`, in its order.
pub const KNOWN_DIFFERENCES: &[KnownDifference] = &[
    KnownDifference {
        name: "numeric_range",
        title: "Numbers beyond 38 digits raise an error in DuckDB",
        differs: "Inside a calculation Snowflake can hold numbers wider than 38 digits; DuckDB \
                  cannot. A SUM or AVG total, a sum, difference or product beyond 38 digits, or \
                  a division whose exact working value is too large raises an error in DuckDB. \
                  So does comparing or combining numbers whose shared type would need more than \
                  38 digits, such as NUMBER(38,0) with NUMBER(10,5), when a value does not fit.",
        fine_when: "Totals, working values and compared values stay within 38 digits.",
        wrong_when: "A value goes beyond 38 digits: DuckDB raises an error where Snowflake \
                     returns a value. It never returns a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "json_fidelity",
        title: "JSON that DuckDB might read differently raises an error",
        differs: "DuckDB's JSON keeps decimal numbers as doubles, accepts a different set of \
                  relaxed spellings and repeated keys, and keeps keys in input order. Snowflake \
                  keeps exact decimals, reads single quotes, NaN and undefined, rejects repeated \
                  keys and writes keys sorted. The converted SQL writes objects with keys sorted \
                  and strings escaped as Snowflake does.",
        fine_when: "Both engines read the JSON the same way; the converted SQL checks every \
                    document.",
        wrong_when: "A document DuckDB might read differently, an object whose sorted keys would \
                     change how a number is written, a FLATTEN path through keys with unusual \
                     characters, or a URL in a shape PARSE_URL has not been checked against: the \
                     converted SQL raises an error where Snowflake returns a value. It never \
                     returns a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "user_override",
        title: "Your replacement for a Snowflake function is trusted to match it",
        differs: "A replacement swaps a Snowflake function for your own DuckDB SQL. SQLQuack \
                  cannot tell whether your SQL gives what the function it replaces gives.",
        fine_when: "Your DuckDB SQL gives the same result as the Snowflake function for every \
                    input.",
        wrong_when: "Your SQL differs from the Snowflake function for some input: those calls \
                     return a different value, with no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "session_timezone_utc",
        title: "Results that depend on the session time zone need UTC",
        differs: "Converted SQL is proven only where both Snowflake and DuckDB run their sessions \
                  in UTC. Some results depend on the session time zone: a DATE, a timestamp \
                  without a time zone, or timestamp text without an offset read as a zoned or \
                  local timestamp; a local timestamp's wall time, date or parts; and \
                  CURRENT_DATE and CURRENT_TIMESTAMP.",
        fine_when: "Both the Snowflake and the DuckDB session use the UTC time zone.",
        wrong_when: "Either session uses another time zone: such a statement returns a different \
                     value, with no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "hash",
        title: "HASH gives a different value in DuckDB",
        differs: "DuckDB computes its own hash, never Snowflake's. Equal inputs still get equal \
                  hashes within one run.",
        fine_when: "Joins, grouping, removing duplicates and spotting changed rows by hash.",
        wrong_when: "Anything that uses the hash value itself: hashes stored and compared with \
                     ones Snowflake computed, hash values written in expected test output, \
                     HASH(x) % n for buckets or samples, and sorting by a hash. A wrong hash \
                     looks like a right one, with no error.",
        failure: DivergenceFailure::Value,
    },
];
