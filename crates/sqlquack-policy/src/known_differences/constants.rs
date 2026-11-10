//! Every known difference, described in plain English.

use crate::known_differences::models::KnownDifference;
use crate::policy::models::DivergenceFailure;

/// One entry per `[divergences.<name>]` of the evidence policy, in its order.
pub const KNOWN_DIFFERENCES: &[KnownDifference] = &[
    KnownDifference {
        name: "numeric_range",
        title: "Numbers beyond 38 digits raise an error in DuckDB",
        differs: "Inside a calculation Snowflake can hold numbers wider than 38 digits, but \
                  DuckDB cannot.",
        details: Some(
            "A SUM or AVG total, a sum, difference or product beyond 38 digits, or a \
                       division whose exact working value is too large raises an error in DuckDB. \
                       So does comparing or combining numbers whose shared type would need more \
                       than 38 digits, such as NUMBER(38,0) with NUMBER(10,5), when a value does \
                       not fit.",
        ),
        fine_when: "Totals and working values stay within 38 digits.",
        wrong_when: "A value goes beyond 38 digits: DuckDB raises an error where Snowflake \
                     returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "text_similarity",
        title: "JAROWINKLER_SIMILARITY raises an error for some text",
        differs: "Snowflake's JAROWINKLER_SIMILARITY ignores case and counts characters. How it \
                  folds letters outside ASCII has not been checked against Snowflake.",
        details: Some(
            "The converted SQL also raises an error for a pair of texts holding more \
                       than 127 distinct characters, which it cannot encode.",
        ),
        fine_when: "Text whose cased letters are all ASCII.",
        wrong_when: "Text holding a cased letter outside ASCII, such as É: DuckDB raises an error \
                     where Snowflake returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "array_sort_order",
        title: "Sorting mixed VARIANT values can raise an error",
        differs: "Snowflake sorts VARIANT values by type, then by value. How it orders two \
                  different objects, two different arrays, or numbers such as 1 and 1.0 has not \
                  been checked against Snowflake.",
        details: Some(
            "ARRAY_SORT puts booleans first, then numbers by value, strings by their \
                       bytes, objects, arrays, and JSON nulls last; ARRAY_MAX and ARRAY_MIN take \
                       its last and first elements. MAX of VARIANT takes the largest number or \
                       the bytewise largest string, ranks a string above a number, and raises an \
                       error for other mixes.",
        ),
        fine_when: "Arrays without such pairs, and MAX of VARIANT numbers or strings.",
        wrong_when: "An array holding such a pair, or MAX of another mix of VARIANT values: \
                     DuckDB raises an error where Snowflake returns a value, never a different \
                     value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "case_folding",
        title: "ILIKE raises an error for some letters outside ASCII",
        differs: "Snowflake's ILIKE matches letters by their lower case, with a few exceptions \
                  such as the Kelvin sign. Letters whose case folding differs from their lower \
                  case have not been checked against Snowflake.",
        details: Some(
            "Snowflake folds the long s onto s and the Kelvin sign onto k. Letters \
                       whose Unicode case folding differs from their lower case, or whose lower \
                       case DuckDB writes differently, raise an error rather than guess whether \
                       they match.",
        ),
        fine_when: "Text without such letters.",
        wrong_when: "Text holding such a letter: DuckDB raises an error where Snowflake returns a \
                     value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "json_fidelity",
        title: "JSON that DuckDB might read differently raises an error",
        differs: "DuckDB reads JSON numbers, spellings and repeated keys differently from \
                  Snowflake. The converted SQL checks every document and writes objects as \
                  Snowflake does.",
        details: Some(
            "DuckDB's JSON keeps decimal numbers as doubles, accepts a different set of \
                       relaxed spellings and repeated keys, and keeps keys in input order. \
                       Snowflake keeps exact decimals, reads single quotes, NaN and undefined, \
                       rejects repeated keys and writes keys sorted. The converted SQL also \
                       raises an error for an object whose sorted keys would change how a number \
                       is written, a FLATTEN path through keys with unusual characters, and a URL \
                       in a shape PARSE_URL has not been checked against.",
        ),
        fine_when: "Both engines read the JSON the same way.",
        wrong_when: "A document DuckDB might read differently: DuckDB raises an error where \
                     Snowflake returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "variant_chain_type",
        title: "A FLOAT in a VARIANT column can lose its type between models",
        differs: "A VARIANT column built from a FLOAT is written as JSON text, so a model reading \
                  it back loses the FLOAT's type. A check meant to catch this has no test showing \
                  it works.",
        details: Some(
            "The model building the column keeps the FLOAT's type and writes the number \
                       as Snowflake spells it, with an exponent. A model reading the column back \
                       gets JSON. The check on VARIANT columns of other relations is meant to \
                       raise an error for any number with an exponent.",
        ),
        fine_when: "The column is stored as a DuckDB VARIANT, which keeps the type.",
        wrong_when: "A model reads a FLOAT back from such a column stored as JSON text: it can \
                     get a different value, with no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "source_variant_json",
        title: "VARIANT columns of source tables are trusted to hold plain JSON",
        differs: "Snowflake keeps a date, time, timestamp or binary value inside a VARIANT as its \
                  own type, but DuckDB's JSON holds only its text. Comparisons of VARIANT values \
                  from a source table rely on them holding plain JSON.",
        details: Some(
            "Comparing VARIANTs, PARTITION BY a VARIANT, ARRAY_CONTAINS, \
                       ARRAY_POSITION, ARRAY_EXCEPT, ANY_VALUE, MAX_BY, MIN_BY and ARRAY_AGG \
                       without WITHIN GROUP tell VARIANT values apart by their text. A column the \
                       schema marks as holding typed values is refused instead.",
        ),
        fine_when: "The source column holds values loaded from JSON.",
        wrong_when: "Something outside the project wrote typed values into the column: they \
                     compare equal to their text, with no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "project_udf",
        title: "Your Python functions are trusted to run locally as in Snowflake",
        differs: "A project Python function runs its handler in a local Python process. SQLQuack \
                  cannot tell whether it returns what it returns in Snowflake.",
        details: Some(
            "The schema declares how each function treats repeated calls and NULL \
                       input. Vectorized handlers are not supported.",
        ),
        fine_when: "The handler gives the same result for the same input, whatever the Python \
                    version, packages or environment.",
        wrong_when: "The handler's result depends on its Python version, packages or environment: \
                     it returns a different value, with no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "user_override",
        title: "Your replacement for a Snowflake function is trusted to match it",
        differs: "A replacement swaps a Snowflake function for your own DuckDB SQL. SQLQuack \
                  cannot tell whether it gives what the function it replaces gives.",
        details: None,
        fine_when: "Your SQL gives the same result as the Snowflake function for every input.",
        wrong_when: "Your SQL differs for some input: those calls return a different value, with \
                     no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "python_value_crossing",
        title: "Some Python values raise an error when crossing to or from SQL",
        differs: "Values cross between SQL and a Python handler only in the shapes checked \
                  against Snowflake. SQLQuack's runtime raises an error for any other shape.",
        details: Some(
            "Inside a VARIANT, a Decimal, a date, a naive datetime with a fraction of a \
                       second, or bytes becomes its str() text, and an int key becomes its \
                       digits. The runtime raises an error for a float, a datetime with a time \
                       zone or without a fraction of a second, or a Decimal in exponent form \
                       inside a VARIANT; a result of another Python type than the declared one; a \
                       Decimal beyond the declared scale or in exponent form; and an int beyond \
                       2^53 returned as FLOAT.",
        ),
        fine_when: "Handlers take and return values in their declared types and checked shapes.",
        wrong_when: "A value in another shape: DuckDB raises an error where Snowflake returns a \
                     value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "declared_offset",
        title: "TIMESTAMP_TZ source columns are trusted to hold their declared offset",
        differs: "DuckDB does not keep a TIMESTAMP_TZ value's own offset. Where the schema \
                  declares a column's offset as +00:00, its wall time, date and parts are worked \
                  out for that offset.",
        details: Some("Only +00:00 can be declared."),
        fine_when: "Every value in the column has the +00:00 offset.",
        wrong_when: "A value has another offset: its wall time, date or parts come out different, \
                     with no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "session_timezone_utc",
        title: "Results that depend on the session time zone need UTC",
        differs: "Converted SQL matches Snowflake only when both sessions use UTC. Some results \
                  depend on the session time zone.",
        details: Some(
            "These are a DATE, a timestamp without a time zone, or timestamp text \
                       without an offset read as a zoned or local timestamp, including \
                       CONVERT_TIMEZONE with two arguments; a local timestamp read as a wall \
                       time, a date part, VARIANT text or a zoned timestamp; and CURRENT_DATE and \
                       CURRENT_TIMESTAMP.",
        ),
        fine_when: "Both the Snowflake and the DuckDB session use UTC.",
        wrong_when: "Either session uses another time zone: such results come out different, with \
                     no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "percentile_interpolation",
        title: "PERCENTILE_CONT and MEDIAN can raise an error instead of interpolating",
        differs: "Snowflake's PERCENTILE_CONT rounds its last digits in a way no formula \
                  reproduces, and its MEDIAN of NUMBER loses digits for wide values.",
        details: Some(
            "The converted SQL returns the value where the percentile lands on a row or \
                       between equal values. For MEDIAN of a NUMBER of scale s, it returns the \
                       exact median where both middle values are below 10^(12-s).",
        ),
        fine_when: "The percentile lands on a row or between equal values.",
        wrong_when: "PERCENTILE_CONT would interpolate between different values, or MEDIAN's \
                     middle values are too wide: DuckDB raises an error where Snowflake returns a \
                     value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "time_zone_spelling",
        title: "Time zone names given per row must be spelled as Snowflake accepts them",
        differs: "Snowflake reads a time zone name exactly, case included, and rejects spellings \
                  such as 'utc' that DuckDB accepts.",
        details: Some(
            "Zone names given per row convert only for names Snowflake has been checked \
                       to accept.",
        ),
        fine_when: "Each row's zone name is one Snowflake has been checked to accept.",
        wrong_when: "A row's zone name is spelled another way: DuckDB raises an error where \
                     Snowflake may return a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "mode_of_no_rows",
        title: "MODE over no rows raises an error",
        differs: "MODE without GROUP BY over no rows returns zero in some Snowflake plans and \
                  NULL in others.",
        details: None,
        fine_when: "The input has at least one row, or the query uses GROUP BY.",
        wrong_when: "MODE without GROUP BY over no rows: DuckDB raises an error where Snowflake \
                     returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "timestamp_tz_offset",
        title: "Zoned timestamp text with a non-UTC offset raises an error",
        differs: "Text read as a TIMESTAMP_TZ carries its offset, which DuckDB does not keep. Its \
                  wall time, date and parts would come out as UTC's.",
        details: None,
        fine_when: "The text's offset is +00:00, Z or absent.",
        wrong_when: "Any other offset: DuckDB raises an error where Snowflake returns a value, \
                     never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "date_text_spelling",
        title: "Dates and times in unusual spellings raise an error",
        differs: "Snowflake reads more date and time spellings than have been checked against \
                  Snowflake. Text in an unchecked spelling raises an error rather than guess.",
        details: Some(
            "TRY_TO_DATE and TRY_CAST to DATE return NULL only for empty text, \
                       impossible values in checked spellings, and spellings Snowflake has been \
                       checked to reject. With a literal format, Snowflake also reads unpadded \
                       fields and, for some formats, surrounding spaces, so only text written \
                       exactly in the format's digits and separators converts. Text read as a \
                       TIME follows the same pattern.",
        ),
        fine_when: "Text in a spelling checked against Snowflake.",
        wrong_when: "Any other spelling: DuckDB raises an error where Snowflake may return a \
                     value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "variant_typed_values",
        title: "Dates and times inside a VARIANT are stored as Snowflake's text",
        differs: "In Snowflake a date, timestamp or time inside a VARIANT keeps its type, which \
                  DuckDB's JSON cannot. The converted SQL stores the text Snowflake writes for it \
                  instead.",
        details: Some(
            "The text is YYYY-MM-DD or YYYY-MM-DD HH:MI:SS.FFF, with a ' Z' suffix for \
                       a TIMESTAMP_LTZ in UTC. TYPEOF and IS_DATE-style checks of a string in \
                       such a text raise an error rather than answer VARCHAR or false. A \
                       TIMESTAMP_TZ, TIME or BINARY inside a VARIANT is refused.",
        ),
        fine_when: "The text keeps every digit and offset of the value.",
        wrong_when: "The text would drop digits or an offset: DuckDB raises an error where \
                     Snowflake returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "timestamp_range",
        title: "Timestamps outside 1677 to 2262 raise an error in DuckDB",
        differs: "DuckDB's nanosecond timestamps span 1677-09-21 to 2262-04-11, but Snowflake's \
                  go beyond.",
        details: Some(
            "This affects TIMESTAMP_NTZ with 7 to 9 fraction digits. TO_TIMESTAMP of an \
                       epoch count outside the years 1 to 9999, which Snowflake returns wrapped \
                       or beyond them, raises an error too.",
        ),
        fine_when: "Timestamps stay within 1677-09-21 to 2262-04-11.",
        wrong_when: "A value outside that range: DuckDB raises an error where Snowflake returns a \
                     value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "timestamp_tz_submicro",
        title: "Zoned timestamps finer than a microsecond raise an error in DuckDB",
        differs: "DuckDB's zoned timestamps hold microseconds, where Snowflake's hold \
                  nanoseconds.",
        details: None,
        fine_when: "Values have no part below a microsecond.",
        wrong_when: "A value has a part below a microsecond: DuckDB raises an error where \
                     Snowflake returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "float_text",
        title: "A FLOAT at the top of a VARIANT written as text raises an error",
        differs: "Snowflake writes such a FLOAT as Java does when it works out a constant ahead \
                  of time, and as C's %g does otherwise. The conversion cannot tell which.",
        details: Some(
            "Elsewhere Snowflake writes a FLOAT as text with 10 to 15 significant \
                       digits, depending on its whole digits, as C's %g does, and the converted \
                       SQL does the same.",
        ),
        fine_when: "A FLOAT written as text directly, not from the top level of a VARIANT.",
        wrong_when: "A FLOAT at the top level of a VARIANT cast to text: DuckDB raises an error \
                     where Snowflake returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "regexp_pattern_syntax",
        title: "Regular expressions from a column can raise an error",
        differs: "A pattern from a column cannot be checked before the query runs. Only common \
                  regular expression syntax has been checked to behave the same in Snowflake and \
                  DuckDB.",
        details: Some(
            "The checked syntax is literal characters, '.', groups, alternation, plain \
                       bracket expressions, single '*', '+' and '?', and anchors at the ends. How \
                       REGEXP_COUNT, REGEXP_INSTR, REGEXP_REPLACE and REGEXP_SUBSTR treat empty \
                       matches has not been checked against Snowflake, so a pattern that matches \
                       empty text raises an error.",
        ),
        fine_when: "Column patterns use only the checked syntax.",
        wrong_when: "A column pattern uses other syntax, such as escapes or counted repetition: \
                     DuckDB raises an error where Snowflake returns a value, never a different \
                     value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "float_sum_order",
        title: "Sums of FLOAT values raise an error unless every order gives the same total",
        differs: "Snowflake adds FLOAT values in an order DuckDB does not share, so totals can \
                  differ in the last digits.",
        details: Some(
            "Snowflake gives 0.1 + 0.2 + 0.3 as 0.6 and 1e16 + 1 - 1e16 as 0, neither \
                       in input order nor exactly. The converted SQL sums only where every order \
                       gives the exact total: values that are whole multiples of one power of \
                       two, with a small enough total. SUM, VARIANCE, STDDEV and the COVAR \
                       functions are affected.",
        ),
        fine_when: "Every order of addition gives the exact total.",
        wrong_when: "Any other FLOAT values: DuckDB raises an error where Snowflake returns a \
                     value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "extreme_key_ties",
        title: "MAX_BY and MIN_BY raise an error when tied rows differ",
        differs: "MAX_BY and MIN_BY pick one row among rows tied on the key, and which one \
                  Snowflake picks has not been checked.",
        details: None,
        fine_when: "Every row tied on the key holds the same value.",
        wrong_when: "Tied rows hold different values: DuckDB raises an error where Snowflake \
                     returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "mode_ties",
        title: "MODE raises an error when several values tie",
        differs: "MODE returns one of the values tied for most frequent, and DuckDB may choose \
                  another than Snowflake.",
        details: Some(
            "Both engines count -0 and 0 as one value whose sign either may keep, so a \
                       FLOAT group holding both raises an error too.",
        ),
        fine_when: "Exactly one value is the most frequent.",
        wrong_when: "Several values tie: DuckDB raises an error where Snowflake returns a value, \
                     never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "array_agg_order",
        title: "ARRAY_AGG raises an error where an undefined order could show",
        differs: "ARRAY_AGG without WITHIN GROUP, or with ties on its keys, returns values in an \
                  order neither engine defines.",
        details: Some(
            "The array converts where it holds at most one distinct value, where tied \
                       rows hold equal values, or where its order cannot show, such as when it \
                       only feeds ARRAY_SIZE, ARRAY_SORT, ARRAY_CONTAINS or the array set \
                       functions. Anywhere the order could show, including as an ORDER BY or \
                       GROUP BY key, it raises an error.",
        ),
        fine_when: "The order of the values cannot show in the result.",
        wrong_when: "The order of different values could show: DuckDB raises an error where \
                     Snowflake returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "listagg_order",
        title: "LISTAGG raises an error when tied rows differ",
        differs: "LISTAGG WITHIN GROUP joins rows tied on every ORDER BY key in an order neither \
                  engine defines.",
        details: None,
        fine_when: "Tied rows hold equal values, or the value itself is the only ORDER BY key.",
        wrong_when: "Tied rows hold different values: DuckDB raises an error where Snowflake \
                     returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "text_number_scale",
        title: "Text compared with a NUMBER must fit the NUMBER's scale",
        differs: "Snowflake reads text compared with a NUMBER as a number. How it rounds text \
                  with more fraction digits or an exponent has not been checked against \
                  Snowflake.",
        details: None,
        fine_when: "The text holds a number exact at the NUMBER's scale.",
        wrong_when: "Other numeric text: DuckDB raises an error where Snowflake returns a value, \
                     never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "hash",
        title: "HASH gives a different value in DuckDB",
        differs: "DuckDB computes its own hash, never Snowflake's. Equal inputs still get equal \
                  hashes within one run.",
        details: Some(
            "Anything that uses the hash value itself goes wrong: hashes stored and \
                       compared with ones Snowflake computed, hash values in expected test \
                       output, HASH(x) % n for buckets or samples, and sorting by a hash.",
        ),
        fine_when: "Joins, grouping, removing duplicates and spotting changed rows by hash.",
        wrong_when: "Storing, comparing or sorting by the hash value itself: it differs, with no \
                     error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "random",
        title: "RANDOM gives different values in DuckDB",
        differs: "Random values differ from Snowflake's by nature.",
        details: None,
        fine_when: "Anything that needs only random values.",
        wrong_when: "Anything expecting Snowflake's random values: they differ, with no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "uuid",
        title: "UUID_STRING gives different identifiers in DuckDB",
        differs: "Generated identifiers differ from Snowflake's, but each stays unique within one \
                  run.",
        details: None,
        fine_when: "Anything that needs only unique identifiers.",
        wrong_when: "Anything expecting the identifiers Snowflake generated: they differ, with no \
                     error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "sequences",
        title: "SEQ1, SEQ2, SEQ4 and SEQ8 give different values in DuckDB",
        differs: "Sequence values and gaps differ from Snowflake's, but values stay unique within \
                  one run.",
        details: None,
        fine_when: "Anything that needs only unique values.",
        wrong_when: "Anything expecting Snowflake's sequence values or gaps: they differ, with no \
                     error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "initcap_text",
        title: "INITCAP raises an error for text with unusual characters",
        differs: "Snowflake's INITCAP capitalises after whitespace and many punctuation \
                  characters. Only letters, digits, spaces, underscores, hyphens and apostrophes \
                  have been checked against Snowflake.",
        details: Some(
            "A word starting with a character whose upper case is several characters \
                       raises an error too.",
        ),
        fine_when: "Text holds only letters, digits, spaces, underscores, hyphens and \
                    apostrophes.",
        wrong_when: "Text with any other character: DuckDB raises an error where Snowflake \
                     returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "current_time",
        title: "CURRENT_DATE and CURRENT_TIMESTAMP read DuckDB's clock",
        differs: "CURRENT_DATE and CURRENT_TIMESTAMP read the clock, so they differ between \
                  Snowflake and DuckDB and between runs.",
        details: Some(
            "The converted SQL reads DuckDB's clock in the session's time zone, with \
                       Snowflake's types.",
        ),
        fine_when: "The result does not depend on the moment, such as a comparison with a fixed \
                    past date.",
        wrong_when: "The result depends on the moment it runs: it differs, with no error.",
        failure: DivergenceFailure::Value,
    },
    KnownDifference {
        name: "time_submicro",
        title: "TIME values finer than a microsecond raise an error",
        differs: "Snowflake keeps a TIME to nanoseconds, but only TIME values to microseconds \
                  have been checked against Snowflake.",
        details: Some("A TIME literal with a part below a microsecond is refused."),
        fine_when: "TIME values have no part below a microsecond.",
        wrong_when: "Text read as a TIME has a part below a microsecond: DuckDB raises an error \
                     where Snowflake returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "any_value_choice",
        title: "ANY_VALUE raises an error when a group's values differ",
        differs: "ANY_VALUE returns some value of the group, and Snowflake and DuckDB may choose \
                  different ones.",
        details: None,
        fine_when: "Every row of the group holds the same value.",
        wrong_when: "A group holds different values, or NULL beside a value: DuckDB raises an \
                     error where Snowflake returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "float_decimal_digits",
        title: "Rounding a FLOAT to a NUMBER works only for some scales and sizes",
        differs: "Snowflake rounds a FLOAT's exact double value half away from zero when it \
                  becomes a NUMBER. This has been checked only for scales 2, 6 and 8, with up to \
                  ten digits once scaled.",
        details: Some(
            "Snowflake rounds the exact double even where the shortest decimal reading \
                       would round the other way. Other scales are refused, and a value with more \
                       significant digits, where Snowflake may round a shorter decimal reading \
                       instead, raises an error.",
        ),
        fine_when: "A NUMBER of scale 2, 6 or 8, with up to ten digits once scaled.",
        wrong_when: "A value with more significant digits: DuckDB raises an error where Snowflake \
                     returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
    KnownDifference {
        name: "timestamp_text_rounding",
        title: "Nanosecond timestamps written as text can raise an error",
        differs: "A TIMESTAMP_NTZ(9) is written as text with three fraction digits, and whether \
                  Snowflake truncates or rounds the dropped digits has not been checked.",
        details: None,
        fine_when: "The dropped part is below half a millisecond, where truncating and rounding \
                    agree.",
        wrong_when: "The dropped part is half a millisecond or more: DuckDB raises an error where \
                     Snowflake returns a value, never a different value.",
        failure: DivergenceFailure::Loud,
    },
];
