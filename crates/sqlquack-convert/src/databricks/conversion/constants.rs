//! The Databricks conversion rules and the functions they accept.

use sqlquack_diagnostics::failures::models::{ErrorKind, FailureMode};

use crate::databricks::conversion::models::{DatabricksType, Element, Signature};

/// A query block: its source, filter, grouping, ordering and limit.
pub const SELECT_RULE: &str = "rule.databricks.select";
/// The input rows, `VALUES ... AS input(...)`, read as a parenthesised DuckDB `VALUES` list.
pub const FIXTURE_RULE: &str = "rule.databricks.fixture";
/// A column reference.
pub const COLUMN_RULE: &str = "rule.databricks.column";
/// A literal number, string, boolean, date, timestamp or `NULL`.
pub const LITERAL_RULE: &str = "rule.databricks.literal";
/// A cast whose source and target types DuckDB converts the same way.
pub const CAST_RULE: &str = "rule.databricks.cast";
/// `+`, `-` and `*` on numbers, which raise on integer overflow in both engines.
pub const ARITHMETIC_RULE: &str = "rule.databricks.arithmetic";
/// `/`, `%` and `DIV` on numbers, raising on a zero divisor as Databricks does in ANSI mode.
pub const DIVISION_RULE: &str = "rule.databricks.division";
/// `=`, `<>`, `<`, `<=`, `>`, `>=` and `<=>` on values of one type family.
pub const COMPARISON_RULE: &str = "rule.databricks.comparison";
/// `AND`, `OR` and `NOT` on booleans.
pub const BOOLEAN_RULE: &str = "rule.databricks.boolean";
/// `IS [NOT] NULL`.
pub const NULL_TEST_RULE: &str = "rule.databricks.null_test";
/// `[NOT] BETWEEN` on values of one type family.
pub const BETWEEN_RULE: &str = "rule.databricks.between";
/// `[NOT] IN` a list of values of one type family.
pub const IN_LIST_RULE: &str = "rule.databricks.in_list";
/// A searched `CASE` whose branches share one type.
pub const CASE_RULE: &str = "rule.databricks.case";
/// Text Databricks reads as BIGINT the way DuckDB does: a sign and at most 18 digits, between spaces.
pub const WHOLE_NUMBER_TEXT: &str = " *[+-]?[0-9]{1,18} *";
/// Number-like text Databricks rejects as malformed: a fraction, or more digits than a BIGINT holds.
pub const MALFORMED_NUMBER_TEXT: &str = r" *[+-]?([0-9]+\.[0-9]+|0*[1-9][0-9]{19,}) *";
/// A character no number Databricks reads holds, outside digits, signs, the point, whitespace and controls.
pub const NOT_NUMBER_CHARACTER: &str = r"[^0-9+\-.\s\pZ\pC]";
/// What a raise for text holding no number says after the text-not-a-number fragment.
pub const MALFORMED_NUMBER_ERROR: &str = "the text compared with an integer is not a number";
/// The error text compared with an integer raises when no rule shows how Databricks reads it.
pub const UNREAD_NUMBER_ERROR: &str =
    "the text compared with an integer is not one no rule reads as Databricks does";
/// `[NOT] LIKE`, `LIKE ANY` and `LIKE ALL`, a backslash escaping and raising on an escape Databricks rejects.
pub const LIKE_RULE: &str = "rule.databricks.like";
/// The patterns whose every backslash escapes `_`, `%` or a backslash, as an RE2 expression.
pub const LIKE_ESCAPES: &str = r"([^\\]|\\[_%\\])*";
/// The error a pattern Databricks rejects for its escapes raises in DuckDB.
pub const LIKE_ESCAPE_ERROR: &str = "the LIKE pattern has an escape Databricks rejects";
/// `[NOT] ILIKE`, its `ANY` and `ALL` forms: `LIKE` of both sides lowered, raising on a character whose case no rule folds.
pub const ILIKE_RULE: &str = "rule.databricks.ilike";
/// The node kind of `ILIKE`, the `LIKE` that lowers both sides.
pub const ILIKE_KIND: &str = "i_like";
/// The code points below U+0800 and in the emoji blocks that DuckDB's `lower` maps as Java 17's does, as an RE2 class body.
pub const LOWERED_CHARACTERS: &str = r"\x{0}-\x{12f}\x{131}-\x{377}\x{37a}-\x{37f}\x{384}-\x{38a}\x{38c}\x{38e}-\x{3a1}\x{3a4}-\x{52f}\x{531}-\x{556}\x{559}-\x{58a}\x{58d}-\x{58f}\x{591}-\x{5c7}\x{5d0}-\x{5ea}\x{5ef}-\x{5f4}\x{600}-\x{61c}\x{61e}-\x{70d}\x{70f}-\x{74a}\x{74d}-\x{7b1}\x{7c0}-\x{7fa}\x{7fd}-\x{7ff}\x{1f300}-\x{1f6d7}\x{1f6e0}-\x{1f6ec}\x{1f6f0}-\x{1f6fc}\x{1f700}-\x{1f773}\x{1f780}-\x{1f7d8}\x{1f7e0}-\x{1f7eb}\x{1f800}-\x{1f80b}\x{1f810}-\x{1f847}\x{1f850}-\x{1f859}\x{1f860}-\x{1f887}\x{1f890}-\x{1f8ad}\x{1f8b0}-\x{1f8b1}\x{1f900}-\x{1f978}\x{1f97a}-\x{1f9cb}\x{1f9cd}-\x{1fa53}\x{1fa60}-\x{1fa6d}\x{1fa70}-\x{1fa74}\x{1fa78}-\x{1fa7a}\x{1fa80}-\x{1fa86}\x{1fa90}-\x{1faa8}\x{1fab0}-\x{1fab6}\x{1fac0}-\x{1fac2}\x{1fad0}-\x{1fad6}";
/// The code points below U+0800 and in the emoji blocks that DuckDB's `upper` maps as Java 17's does, as an RE2 class body.
pub const UPPERED_CHARACTERS: &str = r"\x{0}-\x{de}\x{e0}-\x{148}\x{14a}-\x{1ef}\x{1f1}-\x{377}\x{37a}-\x{37f}\x{384}-\x{38a}\x{38c}\x{38e}-\x{38f}\x{391}-\x{3a1}\x{3a3}-\x{3af}\x{3b1}-\x{52f}\x{531}-\x{556}\x{559}-\x{586}\x{588}-\x{58a}\x{58d}-\x{58f}\x{591}-\x{5c7}\x{5d0}-\x{5ea}\x{5ef}-\x{5f4}\x{600}-\x{61c}\x{61e}-\x{70d}\x{70f}-\x{74a}\x{74d}-\x{7b1}\x{7c0}-\x{7fa}\x{7fd}-\x{7ff}\x{1f300}-\x{1f6d7}\x{1f6e0}-\x{1f6ec}\x{1f6f0}-\x{1f6fc}\x{1f700}-\x{1f773}\x{1f780}-\x{1f7d8}\x{1f7e0}-\x{1f7eb}\x{1f800}-\x{1f80b}\x{1f810}-\x{1f847}\x{1f850}-\x{1f859}\x{1f860}-\x{1f887}\x{1f890}-\x{1f8ad}\x{1f8b0}-\x{1f8b1}\x{1f900}-\x{1f978}\x{1f97a}-\x{1f9cb}\x{1f9cd}-\x{1fa53}\x{1fa60}-\x{1fa6d}\x{1fa70}-\x{1fa74}\x{1fa78}-\x{1fa7a}\x{1fa80}-\x{1fa86}\x{1fa90}-\x{1faa8}\x{1fab0}-\x{1fab6}\x{1fac0}-\x{1fac2}\x{1fad0}-\x{1fad6}";
/// The error a case mapping raises for text holding a character outside the code points it maps as Databricks does.
pub const CASE_MAPPING_ERROR: &str =
    "the text holds a character whose case no rule maps as Databricks does";
/// The characters with a meaning in a Java regular expression outside a class, and `]` and `}`; a pattern without them is text.
pub const REGEX_METACHARACTERS: &str = r"\.^$|?*+()[]{}";
/// [`REGEX_METACHARACTERS`] as an RE2 class body.
pub const REGEX_METACHARACTER_CLASS: &str = r"\\.^$|?*+()\[\]{}";
/// The error `RLIKE` raises for a pattern holding a metacharacter, whose Java meaning no rule shows RE2 sharing.
pub const PATTERN_ERROR: &str =
    "the pattern holds a regular-expression metacharacter no rule converts";
/// The error a column replacement raises for `$` or `\`, which Java reads as group references and escapes.
pub const REPLACEMENT_ERROR: &str =
    "the replacement holds $ or \\, which no rule reads as Java does";
/// The error an empty pattern raises over a character above U+FFFF, which Java steps through as two UTF-16 units.
pub const SURROGATE_ERROR: &str =
    "an empty pattern steps through a character above U+FFFF as Java's two UTF-16 units";
/// The error REGEXP_EXTRACT and REGEXP_EXTRACT_ALL raise for a match when the group index is outside the pattern's groups.
pub const GROUP_INDEX_ERROR: &str = "the group index is outside the pattern's groups";
/// The error a pattern ending in `$` raises over text ending in CR LF, where Java's `$` does not match between the two.
pub const LINE_END_ERROR: &str =
    "the text ends in a carriage return and line feed, between which Java's $ does not match";
/// Java's `.`, which matches any character but these line terminators, as an RE2 class.
pub const JAVA_DOT: &str = r"[^\n\r\x{85}\x{2028}\x{2029}]";
/// Java's `\s`, ASCII whitespace including the vertical tab RE2's leaves out, as an RE2 class body.
pub const JAVA_SPACE: &str = r" \t\n\x0B\f\r";
/// Java's final `$`, which also matches before a final line terminator, as RE2 that tests the same text.
pub const JAVA_END: &str = r"(?:\r\n|[\n\r\x{85}\x{2028}\x{2029}])?$";
/// The characters above U+FFFF, which Java counts as two UTF-16 units, as an RE2 class.
pub const SUPPLEMENTARY_CHARACTERS: &str = r"[\x{10000}-\x{10FFFF}]";
/// The largest repeat count RE2 takes.
pub const MAX_REPEAT: u32 = 1000;
/// The largest group index DuckDB's regexp_extract and replacements take.
pub const MAX_GROUP: usize = 9;
/// `ORDER BY` with the null ordering written out: Databricks puts NULLs first ascending.
pub const ORDER_RULE: &str = "rule.databricks.order";
/// `CONCAT` of strings as `||`, NULL when any argument is NULL.
pub const CONCAT_RULE: &str = "rule.databricks.concat";
/// `TRIM ... FROM` as DuckDB's trim, ltrim or rtrim with the characters written out.
pub const TRIM_RULE: &str = "rule.databricks.trim";
/// `&`, `|`, `~`, `^`, which is XOR in Databricks and DuckDB's `xor`, and shifts, which wrap as in Java.
pub const BITWISE_RULE: &str = "rule.databricks.bitwise";
/// A function DuckDB has under the same or another name with the same results.
pub const SAME_FUNCTION_RULE: &str = "rule.databricks.same_function";
/// A function rewritten for DuckDB, such as NVL2 as a CASE or LEFT with its zero length written out.
pub const REWRITE_RULE: &str = "rule.databricks.rewrite";
/// The digest length SHA2 reads as SHA-256.
pub const SHA2_SHA256_BITS: &str = "256";
/// The digest length SHA2 also reads as SHA-256.
pub const SHA2_DEFAULT_BITS: &str = "0";
/// The digest lengths SHA2 computes that DuckDB has no function for.
pub const SHA2_MISSING_BITS: &str = "224, 384, 512";
/// The error SHA2 raises for a digest length DuckDB has no function for.
pub const DIGEST_ERROR: &str = "the SHA2 digest length has no DuckDB function";
/// An aggregate DuckDB computes the same way, its result cast to the type Databricks gives it.
pub const AGGREGATE_RULE: &str = "rule.databricks.aggregate";
/// A date or timestamp function: a field such as DAY or HOUR, or date arithmetic in whole days.
pub const DATE_FUNCTION_RULE: &str = "rule.databricks.date_function";
/// A string function, with Databricks's handling of empty, NULL and out-of-range arguments written out.
pub const STRING_FUNCTION_RULE: &str = "rule.databricks.string_function";
/// A regular-expression function of text or of a constant pattern in the part of Java's syntax RE2 reads alike.
pub const REGEX_RULE: &str = "rule.databricks.regex";
/// A number function such as MOD, CEIL or SIGN, typed and bounded as Databricks types and bounds it.
pub const NUMBER_FUNCTION_RULE: &str = "rule.databricks.number_function";
/// A function taking a 1-based index, such as ELEMENT_AT or SPLIT_PART, raising where Databricks raises.
pub const INDEX_RULE: &str = "rule.databricks.index";
/// What a raise for an index of 0 says after the rejected-argument fragment.
pub const INDEX_ZERO_ERROR: &str = "the index 0 is invalid, as the first element has index 1";
/// What a raise for a SLICE start of 0 says after the rejected-argument fragment.
pub const SLICE_START_ERROR: &str = "the slice start is 0, as the first element has index 1";
/// What a raise for a negative SLICE length says after the rejected-argument fragment.
pub const SLICE_LENGTH_ERROR: &str = "the slice length is negative";
/// What a raise for an index past either end of an array says after the rejected-argument fragment.
pub const INDEX_BOUNDS_ERROR: &str = "the index is outside the array";

/// COUNT, the one aggregate with a `*` argument and a DISTINCT form that changes its result.
pub const COUNT_NAME: &str = "COUNT";
/// The fewest arguments a variadic function such as COALESCE takes.
pub const MIN_LIST_ARGUMENTS: usize = 2;
/// Same-type functions that order their arguments, refused for arrays, whose NULL elements the engines order apart.
pub const ORDERING_FUNCTIONS: &[&str] = &["GREATEST", "LEAST"];
/// Functions whose STRING arguments Databricks casts: the type the recorded cast errors name, and the argument positions it casts; a BIGINT reading of several arguments needs an integer beside the text.
pub const TEXT_READINGS: &[(&str, DatabricksType, &[usize])] = &[
    ("MINUTE", DatabricksType::Timestamp, &[0]),
    ("MONTH", DatabricksType::Date, &[0]),
    ("MONTHNAME", DatabricksType::Date, &[0]),
    ("DATE_TRUNC", DatabricksType::Timestamp, &[1]),
    ("TRUNC", DatabricksType::Date, &[0]),
    ("NVL", DatabricksType::BigInt, &[0, 1]),
    ("IFNULL", DatabricksType::BigInt, &[0, 1]),
    ("NVL2", DatabricksType::BigInt, &[1, 2]),
    ("ZEROIFNULL", DatabricksType::BigInt, &[0]),
    ("PMOD", DatabricksType::BigInt, &[0, 1]),
    ("FACTORIAL", DatabricksType::Int, &[0]),
    ("ELT", DatabricksType::Int, &[0]),
    ("MAKE_DATE", DatabricksType::Int, &[0, 1, 2]),
    ("MAKE_TIMESTAMP", DatabricksType::Int, &[0, 1, 2, 3, 4]),
    ("MAKE_TIMESTAMP_LTZ", DatabricksType::Int, &[0, 1, 2, 3, 4]),
    ("MAKE_TIMESTAMP_NTZ", DatabricksType::Int, &[0, 1, 2, 3, 4]),
    ("TRY_MAKE_TIMESTAMP", DatabricksType::Int, &[0, 1, 2, 3, 4]),
    (
        "TRY_MAKE_TIMESTAMP_LTZ",
        DatabricksType::Int,
        &[0, 1, 2, 3, 4],
    ),
    (
        "TRY_MAKE_TIMESTAMP_NTZ",
        DatabricksType::Int,
        &[0, 1, 2, 3, 4],
    ),
    ("SHIFTLEFT", DatabricksType::Int, &[0, 1]),
    ("SHIFTRIGHT", DatabricksType::Int, &[0, 1]),
    ("SHIFTRIGHTUNSIGNED", DatabricksType::Int, &[0, 1]),
    ("DEGREES", DatabricksType::Double, &[0]),
    ("RADIANS", DatabricksType::Double, &[0]),
    ("SQRT", DatabricksType::Double, &[0]),
    ("CEIL", DatabricksType::Double, &[0]),
    ("CEILING", DatabricksType::Double, &[0]),
    ("FLOOR", DatabricksType::Double, &[0]),
    ("ROUND", DatabricksType::Double, &[0]),
    ("BROUND", DatabricksType::Double, &[0]),
    ("ISNAN", DatabricksType::Double, &[0]),
    ("NANVL", DatabricksType::Double, &[0, 1]),
    ("BIN", DatabricksType::BigInt, &[0]),
    ("CHAR", DatabricksType::BigInt, &[0]),
    ("CHR", DatabricksType::BigInt, &[0]),
    ("FROM_UNIXTIME", DatabricksType::BigInt, &[0]),
    ("MOD", DatabricksType::BigInt, &[0, 1]),
    ("YEAR", DatabricksType::Date, &[0]),
    ("QUARTER", DatabricksType::Date, &[0]),
    ("DAY", DatabricksType::Date, &[0]),
    ("DAYOFMONTH", DatabricksType::Date, &[0]),
    ("DAYOFYEAR", DatabricksType::Date, &[0]),
    ("DAYOFWEEK", DatabricksType::Date, &[0]),
    ("WEEKDAY", DatabricksType::Date, &[0]),
    ("WEEKOFYEAR", DatabricksType::Date, &[0]),
    ("DAYNAME", DatabricksType::Date, &[0]),
    ("LAST_DAY", DatabricksType::Date, &[0]),
    ("ADD_MONTHS", DatabricksType::Date, &[0]),
    ("NEXT_DAY", DatabricksType::Date, &[0]),
    ("DATEDIFF", DatabricksType::Date, &[0, 1]),
    ("HOUR", DatabricksType::Timestamp, &[0]),
    ("SECOND", DatabricksType::Timestamp, &[0]),
    ("MONTHS_BETWEEN", DatabricksType::Timestamp, &[0, 1]),
    ("DATE_DIFF", DatabricksType::Date, &[0, 1]),
];
/// Functions with INT parameters a BIGINT argument narrows to, raising when it does not fit: the argument positions.
pub const INT_PARAMETERS: &[(&str, &[usize])] = &[
    ("FACTORIAL", &[0]),
    ("ELT", &[0]),
    ("MAKE_DATE", &[0, 1, 2]),
    ("MAKE_TIMESTAMP", &[0, 1, 2, 3, 4]),
    ("MAKE_TIMESTAMP_LTZ", &[0, 1, 2, 3, 4]),
    ("MAKE_TIMESTAMP_NTZ", &[0, 1, 2, 3, 4]),
    ("TRY_MAKE_TIMESTAMP", &[0, 1, 2, 3, 4]),
    ("TRY_MAKE_TIMESTAMP_LTZ", &[0, 1, 2, 3, 4]),
    ("TRY_MAKE_TIMESTAMP_NTZ", &[0, 1, 2, 3, 4]),
    ("SHIFTLEFT", &[1]),
    ("SHIFTRIGHT", &[1]),
    ("SHIFTRIGHTUNSIGNED", &[1]),
    ("BIT_GET", &[1]),
    ("GETBIT", &[1]),
    ("SUBSTR", &[1, 2]),
    ("SUBSTRING", &[1, 2]),
    ("LEFT", &[1]),
    ("RIGHT", &[1]),
    ("SPLIT_PART", &[2]),
    ("SLICE", &[1, 2]),
];
/// The error text read as an integer type raises when it is a whole number outside that type.
pub const NARROW_NUMBER_ERROR: &str = "the text read as an integer is outside its type";
/// Text Databricks reads as TRUE in a cast to BOOLEAN, after lowering it.
pub const TRUE_TEXTS: &[&str] = &["t", "true", "y", "yes", "1"];
/// Text Databricks reads as FALSE in a cast to BOOLEAN, after lowering it.
pub const FALSE_TEXTS: &[&str] = &["f", "false", "n", "no", "0"];
/// What a raise for text that no trimming makes a boolean says; it names the cast to BOOLEAN.
pub const BOOLEAN_TEXT_ERROR: &str = "the text cast to BOOLEAN is not a boolean";
/// Text whose ends Databricks's trimming leaves alone: no space or control character first or last.
pub const UNTRIMMED_TEXT: &str = r"(?s)[^\s\pZ\pC](.*[^\s\pZ\pC])?";
/// The error text cast to BOOLEAN raises when no rule shows how Databricks reads it.
pub const BOOLEAN_READING_ERROR: &str = "no rule reads this text as Databricks reads a boolean";
/// Characters outside the base64 alphabet and padding, which Java's MIME decoder skips.
pub const BASE64_ALPHABET: &str = "[^A-Za-z0-9+/=]";
/// The characters of each line BASE64 writes, as Java's MIME encoder breaks them with CR LF.
pub const BASE64_LINE: u8 = 76;
/// Whole base64 units, the last possibly padded.
pub const BASE64_UNITS: &str = "([A-Za-z0-9+/]{4})*([A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?";
/// Base64 without padding whose last unit holds at least one byte.
pub const BASE64_UNPADDED: &str = "([A-Za-z0-9+/]{4})*[A-Za-z0-9+/]{2,3}";
/// The error base64 text raises when Java's decoder rejects it: one character left over, or data after padding.
pub const BASE64_UNIT_ERROR: &str =
    "the base64 text ends in a lone character or holds data after its padding";
/// The error base64 text of any other shape raises.
pub const BASE64_READING_ERROR: &str = "no rule reads this base64 text as Databricks decodes it";
/// Text of hex digits only, which UNHEX reads.
pub const HEX_TEXT: &str = "[0-9A-Fa-f]*";
/// A `%` not followed by two hex digits, which Java's URLDecoder rejects.
pub const URL_MALFORMED_ESCAPE: &str = "%([^0-9A-Fa-f]|.[^0-9A-Fa-f]|.?$)";
/// URL escapes whose digits Java reads with a sign or as non-ASCII digits, which DuckDB does not.
pub const URL_JAVA_DIGITS: &str = r"%(\+|-|[^\x00-\x7f]|.[^\x00-\x7f])";
/// The error an escape in [`URL_JAVA_DIGITS`] raises.
pub const URL_DIGIT_ERROR: &str =
    "the text holds a URL escape whose digits no rule reads as Java does";
/// The error a malformed URL escape raises.
pub const URL_ESCAPE_ERROR: &str = "the text holds a URL escape Databricks cannot decode";
/// The largest finite FLOAT; a DOUBLE beyond it is one no rule narrows as Databricks does.
pub const FLOAT_MAX: &str = "3.4028234663852886e38";
/// The midpoint between FLOAT's largest value and 2^128, from which Java's cast of a DOUBLE rounds to infinity.
pub const FLOAT_OVERFLOW: &str = "3.4028235677973366e38";
/// The error a DOUBLE beyond FLOAT's range raises when cast to FLOAT.
pub const FLOAT_RANGE_ERROR: &str =
    "no rule narrows a DOUBLE just beyond FLOAT's range as Databricks does";
/// The seconds from the epoch within which a BIGINT, DECIMAL or DOUBLE count converts: about 31,000 years, well inside both engines' timestamps.
pub const EPOCH_SECONDS_BOUND: i64 = 1_000_000_000_000;
/// The error an epoch count beyond the bound raises; Databricks reaches further, and saturates DOUBLE seconds.
pub const EPOCH_RANGE_ERROR: &str = "no rule reads an epoch count beyond 31,000 years from 1970";
/// The most fraction digits a DECIMAL count of seconds may have: microseconds, which Databricks keeps exactly.
pub const EPOCH_DECIMAL_SCALE: u8 = 6;
/// Text DuckDB reads as the DOUBLE Databricks casts it to: a signed decimal of at most 30 digits and a two-digit exponent, between spaces.
pub const PLAIN_DOUBLE_TEXT: &str =
    r" *[+-]?([0-9]{1,30}|[0-9]{0,30}\.[0-9]{1,30})([eE][+-]?[0-9]{1,2})? *";
/// Text that may hold a number Databricks reads: a digit, or the words of infinity and NaN.
pub const NUMBER_SHAPED_TEXT: &str = "(?i)[0-9]|inf|nan";
/// What a raise for text with no digit says after the text-not-a-number fragment.
pub const DIGITLESS_NUMBER_ERROR: &str = "the text cast to DOUBLE holds no number";
/// The error text cast to DOUBLE raises when no rule shows how Databricks reads it.
pub const NUMBER_READING_ERROR: &str =
    "the text cast to DOUBLE is not one no rule reads as Databricks does";
/// Text DuckDB reads as the DATE Databricks casts it to: `YYYY-MM-DD` with a year from 1000.
pub const PLAIN_DATE_TEXT: &str = "[1-9][0-9]{3}-[0-9]{2}-[0-9]{2}";
/// Text DuckDB reads as the timestamp Databricks casts it to: `YYYY-MM-DD HH:MM:SS` with an hour below 24 and up to six fraction digits.
pub const PLAIN_TIMESTAMP_TEXT: &str =
    "[1-9][0-9]{3}-[0-9]{2}-[0-9]{2} ([01][0-9]|2[0-3]):[0-9]{2}:[0-9]{2}(\\.[0-9]{1,6})?";
/// What a raise for a timestamp text no timestamp reads says after the timestamp-text fragment.
pub const INVALID_TIMESTAMP_ERROR: &str = "the text is no valid timestamp";
/// What a raise for text with no digit cast to a timestamp says after the timestamp-text fragment.
pub const DIGITLESS_TIMESTAMP_ERROR: &str = "the text read as a timestamp holds no timestamp";
/// The error text cast to a timestamp raises when no rule shows how Databricks reads it.
pub const TIMESTAMP_READING_ERROR: &str = "no rule reads this text as Databricks reads a timestamp";
/// The error fragment a malformed date raises, which the evidence reads as malformed date text.
pub const DATE_TEXT_FRAGMENT: &str = "invalid date field format";
/// The error fragment a malformed timestamp raises, which the evidence reads as malformed timestamp text.
pub const TIMESTAMP_TEXT_FRAGMENT: &str = "invalid timestamp field format";
/// What a raise for text with no digit cast to DATE says after the date-text fragment.
pub const DIGITLESS_DATE_ERROR: &str = "the text read as a date holds no date";
/// The error text cast to DATE raises when no rule shows how Databricks reads it.
pub const DATE_READING_ERROR: &str = "no rule reads this text as Databricks reads a date";
/// The most digits a literal has for one rounding to DOUBLE to match Databricks's exact conversion.
pub const DOUBLE_EXACT_DIGITS: usize = 15;
/// The length of `YYYY-MM-DD`.
pub const DATE_TEXT_LENGTH: usize = 10;
/// The length of `HH:MM:SS`.
pub const CLOCK_TEXT_LENGTH: usize = 8;
/// The most fraction digits of a second both engines keep: microseconds.
pub const MAX_FRACTION_DIGITS: usize = 6;
/// The type the parser keeps as a custom name: a wall-clock timestamp.
pub const TIMESTAMP_NTZ_NAME: &str = "TIMESTAMP_NTZ";
/// The text values both engines read as DOUBLE's special values.
pub const SPECIAL_DOUBLE_TEXTS: &[&str] = &["NaN", "Infinity", "-Infinity"];
/// The most significant bits a FLOAT holds.
pub const FLOAT_MANTISSA_BITS: u32 = 24;
/// The most significant bits a DOUBLE holds.
pub const DOUBLE_MANTISSA_BITS: u32 = 53;
/// The parameter name of the lambdas array rewrites emit.
pub const LAMBDA_ELEMENT: &str = "element";
/// The lambda parameter of an element's 1-based place, such as ARRAY_DISTINCT's.
pub const LAMBDA_INDEX: &str = "slot";
/// The parameter name of the text joined so far in the lambdas that join array elements.
pub const JOINED: &str = "joined";
/// The array elements whose text DuckDB writes as Databricks returns it; strings raise on control characters.
pub const RENDERED_ELEMENTS: &[Element] = &[
    Element::String,
    Element::Boolean,
    Element::TinyInt,
    Element::SmallInt,
    Element::Int,
    Element::BigInt,
];
/// The control characters whose JSON escapes in an array no recording shows Databricks writing.
pub const CONTROL_CHARACTERS: &str = r"[\x00\x08\x0c\x0d]";
/// The error a string array holding a control character raises in DuckDB.
pub const CONTROL_ERROR: &str = "no recording shows how Databricks writes NUL, backspace, form feed or carriage return in an array";
/// How deep a tree may nest for conversion, which recurses once per level.
pub const MAX_TREE_DEPTH: usize = 128;
/// The most digits a Databricks decimal holds.
pub const MAX_DECIMAL_PRECISION: u8 = 38;
/// The decimal type `DECIMAL` without a precision means.
pub const DEFAULT_DECIMAL: DatabricksType = DatabricksType::Decimal {
    precision: 10,
    scale: 0,
};

/// Functions DuckDB computes as Databricks does; case mapping is left out, as Java maps `ß` differently.
pub const SAME_FUNCTIONS: &[(&str, &str, Signature)] = &[
    ("ABS", "abs", Signature::SameNumber),
    ("COALESCE", "coalesce", Signature::SameType),
    ("GREATEST", "greatest", Signature::SameType),
    ("LEAST", "least", Signature::SameType),
    ("LTRIM", "ltrim", Signature::TrimSpaces),
    ("RTRIM", "rtrim", Signature::TrimSpaces),
    ("TRIM", "trim", Signature::TrimSpaces),
];

/// An overflow, which Databricks raises as `ARITHMETIC_OVERFLOW` or `CAST_OVERFLOW`.
const OVERFLOW: FailureMode = FailureMode {
    kind: ErrorKind::NumericOverflow,
    snowflake: &[],
};
/// A division by zero, which Databricks raises as `DIVIDE_BY_ZERO` or `REMAINDER_BY_ZERO`.
const DIVISION: FailureMode = FailureMode {
    kind: ErrorKind::DivisionByZero,
    snowflake: &[],
};
/// Text no number reads, a malformed cast to a number.
const NUMBER_TEXT: FailureMode = FailureMode {
    kind: ErrorKind::NumberText,
    snowflake: &[],
};
/// Text no date reads, a malformed cast to DATE.
const DATE_TEXT: FailureMode = FailureMode {
    kind: ErrorKind::DateText,
    snowflake: &[],
};
/// Text no timestamp reads, a malformed cast to a timestamp.
const TIMESTAMP_TEXT: FailureMode = FailureMode {
    kind: ErrorKind::TimestampText,
    snowflake: &[],
};
/// Text no boolean reads, a malformed cast to BOOLEAN.
const BOOLEAN_TEXT: FailureMode = FailureMode {
    kind: ErrorKind::BooleanText,
    snowflake: &[],
};
/// An argument the function rejects, such as an index of 0.
const ARGUMENT: FailureMode = FailureMode {
    kind: ErrorKind::InvalidArgument,
    snowflake: &[],
};
/// How each rule's DuckDB SQL may fail as Databricks does; Databricks errors carry no Snowflake code.
pub const FAILURE_MODES: &[(&str, &[FailureMode])] = &[
    (ARITHMETIC_RULE, &[OVERFLOW]),
    (SAME_FUNCTION_RULE, &[OVERFLOW]),
    (DIVISION_RULE, &[DIVISION, OVERFLOW]),
    (AGGREGATE_RULE, &[OVERFLOW]),
    (
        DATE_FUNCTION_RULE,
        &[OVERFLOW, ARGUMENT, DATE_TEXT, TIMESTAMP_TEXT],
    ),
    (COMPARISON_RULE, &[NUMBER_TEXT]),
    (BETWEEN_RULE, &[NUMBER_TEXT]),
    (IN_LIST_RULE, &[NUMBER_TEXT]),
    (CASE_RULE, &[NUMBER_TEXT]),
    (STRING_FUNCTION_RULE, &[OVERFLOW, ARGUMENT]),
    (REGEX_RULE, &[ARGUMENT]),
    (NUMBER_FUNCTION_RULE, &[OVERFLOW, DIVISION, ARGUMENT]),
    (INDEX_RULE, &[ARGUMENT]),
    (
        CAST_RULE,
        &[
            OVERFLOW,
            NUMBER_TEXT,
            DATE_TEXT,
            TIMESTAMP_TEXT,
            BOOLEAN_TEXT,
        ],
    ),
];

/// The largest BIGINT, where Java's cast of a larger DOUBLE saturates.
pub const BIGINT_MAX: &str = "9223372036854775807";
/// The smallest BIGINT, where Java's cast of a smaller DOUBLE saturates.
pub const BIGINT_MIN: &str = "-9223372036854775808";
/// The digits CONV reads and writes, upper case, by value.
pub const BASE_DIGITS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
/// [`BASE_DIGITS`] with lower-case letters, which CONV also reads.
pub const LOWER_BASE_DIGITS: &str = "0123456789abcdefghijklmnopqrstuvwxyz";
/// 2^64, one more than the largest unsigned 64-bit value CONV computes in.
pub const UNSIGNED_SPAN: &str = "18446744073709551616";
/// 2^63, from which CONV's unsigned 64-bit value is negative as a signed one.
pub const SIGN_BIT: &str = "9223372036854775808";
/// The lambda parameter of a running total, such as CONV's value as it reads each digit.
pub const LAMBDA_TOTAL: &str = "total";
/// The largest argument FACTORIAL takes; it returns NULL above it, as 21! exceeds BIGINT.
pub const FACTORIAL_MAX: u8 = 20;

/// The modulus CHR takes its code by: it returns the character of the code's low byte.
pub const CHR_MODULUS: u16 = 256;
/// The pad LPAD and RPAD use when none is given, a space.
pub const DEFAULT_PAD: &str = "' '";
/// The length SUBSTR takes when none is given, INT's largest.
pub const SUBSTRING_LENGTH: &str = "2147483647";
/// The years whose every day both engines hold as a date, well inside DuckDB's and Java's day counts.
pub const DATE_YEARS: (i32, i32) = (-5_000_000, 5_000_000);
/// The largest year Java's dates take; Databricks raises for a field past it.
pub const JAVA_YEAR_MAX: i64 = 999_999_999;
/// What a raise for a date or time field out of bounds says after the rejected-argument fragment.
pub const FIELD_BOUNDS_ERROR: &str = "a date or time field is out of bounds";
/// The years whose every second both engines hold as a timestamp; the year on each side holds only part.
pub const TIMESTAMP_YEARS: (i32, i32) = (-290_307, 294_246);
/// Microseconds per second.
pub const MICROS_PER_SECOND: i64 = 1_000_000;
/// Microseconds per millisecond.
pub const MICROS_PER_MILLI: i64 = 1_000;
/// The units TRUNC takes, by DuckDB's date part; weeks start on Monday in both engines.
pub const DATE_TRUNC_UNITS: &[(&str, &[&str])] = &[
    ("year", &["YEAR", "YYYY", "YY"]),
    ("quarter", &["QUARTER"]),
    ("month", &["MONTH", "MM", "MON"]),
    ("week", &["WEEK"]),
];
/// The units DATE_TRUNC takes, by DuckDB's date part.
pub const TIMESTAMP_TRUNC_UNITS: &[(&str, &[&str])] = &[
    ("year", &["YEAR", "YYYY", "YY"]),
    ("quarter", &["QUARTER"]),
    ("month", &["MONTH", "MM", "MON"]),
    ("week", &["WEEK"]),
    ("day", &["DAY", "DD"]),
    ("hour", &["HOUR"]),
    ("minute", &["MINUTE"]),
    ("second", &["SECOND"]),
    ("millisecond", &["MILLISECOND"]),
    ("microsecond", &["MICROSECOND"]),
];
/// Seconds per day.
pub const SECONDS_PER_DAY: i64 = 86_400;
/// The seconds MONTHS_BETWEEN counts a month's fraction by, 31 days, as a DOUBLE.
pub const SECONDS_PER_MONTH: &str = "2.6784e6";
/// What MONTHS_BETWEEN scales its result by before rounding it to a whole number: 8 places.
pub const MONTHS_BETWEEN_SCALE: &str = "1e8";
/// 2^63, at or above which a Java cast of a DOUBLE to BIGINT saturates.
pub const BIGINT_SPAN_DOUBLE: &str = "9.223372036854775808e18";
/// The names NEXT_DAY reads, upper-cased, for each weekday from Monday, ISO day 1.
pub const WEEKDAY_NAMES: &[&[&str]] = &[
    &["MO", "MON", "MONDAY"],
    &["TU", "TUE", "TUESDAY"],
    &["WE", "WED", "WEDNESDAY"],
    &["TH", "THU", "THURSDAY"],
    &["FR", "FRI", "FRIDAY"],
    &["SA", "SAT", "SATURDAY"],
    &["SU", "SUN", "SUNDAY"],
];
/// What a raise for a weekday name NEXT_DAY does not read says after the rejected-argument fragment.
pub const WEEKDAY_ERROR: &str = "the text names no day of the week";
/// What a raise for a bit position outside the value's width says after the rejected-argument fragment.
pub const BIT_POSITION_ERROR: &str = "the bit position is outside the value";
/// The seconds since 1970 of 0001-01-01 00:00:00 and 9999-12-31 23:59:59, whose years Java writes as four digits.
pub const UNIX_SECONDS_YEARS_1_TO_9999: (i64, i64) = (-62_135_596_800, 253_402_300_799);
/// The Java datetime pattern letters DATE_FORMAT takes, by count, with the `strftime` field that writes them alike.
pub const DATE_PATTERN_LETTERS: &[(&str, &str)] = &[
    ("yyyy", "%Y"),
    ("yy", "%y"),
    ("MMMM", "%B"),
    ("MMM", "%b"),
    ("MM", "%m"),
    ("M", "%-m"),
    ("dd", "%d"),
    ("d", "%-d"),
    ("DDD", "%j"),
    ("EEEE", "%A"),
    ("EEE", "%a"),
    ("E", "%a"),
    ("a", "%p"),
    ("HH", "%H"),
    ("H", "%-H"),
    ("hh", "%I"),
    ("h", "%-I"),
    ("mm", "%M"),
    ("m", "%-M"),
    ("ss", "%S"),
    ("s", "%-S"),
    ("SSS", "%g"),
    ("SSSSSS", "%f"),
];
/// The error FROM_UNIXTIME raises for a time whose year no rule shows Databricks writing.
pub const UNFORMATTED_YEAR_ERROR: &str =
    "the year of the time is outside those no rule shows Databricks writing";
/// 2^53, below which a sum of integers as DOUBLE is exact in any order, as Databricks sums them.
pub const DOUBLE_EXACT_INTEGERS: &str = "9007199254740992";
/// The error AVG of integers raises for a total too large to sum exactly as DOUBLE.
pub const INEXACT_SUM_ERROR: &str =
    "the total is too large for an exact DOUBLE sum no rule shows Databricks matching";
/// The date day counts start from.
pub const EPOCH_DATE: &str = "1970-01-01";
/// 2^31: an INT day count wraps past it, as Databricks's DATE_ADD does.
pub const INT_RANGE: u64 = 2_147_483_648;
/// 2^32, the span of INT values.
pub const INT_SPAN: u64 = 4_294_967_296;

/// The aggregates converted, with the DuckDB function computing each.
pub const AGGREGATES: &[(&str, &str)] = &[
    ("COUNT", "count"),
    ("MIN", "min"),
    ("MAX", "max"),
    ("SUM", "sum"),
    ("BOOL_AND", "bool_and"),
    ("EVERY", "bool_and"),
    ("BOOL_OR", "bool_or"),
    ("SOME", "bool_or"),
    ("ANY", "bool_or"),
    ("COUNT_IF", "count_if"),
    ("AVG", "avg"),
    ("MEAN", "avg"),
    ("TRY_AVG", "avg"),
    ("MEDIAN", "median"),
    ("BIT_AND", "bit_and"),
    ("BIT_OR", "bit_or"),
    ("BIT_XOR", "bit_xor"),
];

/// The functions named after a type, which cast their one argument to it.
pub const CAST_FUNCTIONS: &[(&str, DatabricksType)] = &[
    ("BOOLEAN", DatabricksType::Boolean),
    ("TINYINT", DatabricksType::TinyInt),
    ("SMALLINT", DatabricksType::SmallInt),
    ("INT", DatabricksType::Int),
    ("BIGINT", DatabricksType::BigInt),
    ("FLOAT", DatabricksType::Float),
    ("DOUBLE", DatabricksType::Double),
    ("DECIMAL", DEFAULT_DECIMAL),
    ("STRING", DatabricksType::String),
    ("DATE", DatabricksType::Date),
    ("TIMESTAMP", DatabricksType::Timestamp),
];
