//! Databricks statements converted to DuckDB, and the ones refused.

use crate::databricks::analysis::main::analyze::analyze;
use crate::databricks::conversion::models::{DatabricksType, Element};
use sqlquack_diagnostics::refusals::models::RefusalCategory;

use crate::databricks::conversion::tests::test_types::{
    CategoryTestCase, ConvertTestCase, RefuseTestCase,
};

const FIXTURE: &str = "WITH input AS (SELECT * FROM VALUES (1, CAST(-7 AS INT), CAST('a' AS STRING)), \
     (2, CAST(NULL AS INT), CAST(NULL AS STRING)) AS input(ordinal, x1, s)) ";

#[test]
fn given_a_query_over_the_fixture_when_converted_then_duckdb_sql_keeps_databricks_types() {
    let test_cases = [
        ConvertTestCase {
            description: "the fixture as a VALUES list, a same-named function, NULLs first ascending",
            query: "SELECT ABS ( x1 ) AS r FROM input ORDER BY ordinal",
            expected_sql: &[
                "WITH \"input\" AS (SELECT * FROM (VALUES (1, CAST(-7 AS INTEGER), CAST('a' AS VARCHAR)), \
                 (2, CAST(NULL AS INTEGER), CAST(NULL AS VARCHAR))) AS \"input\"(\"ordinal\", \"x1\", \"s\")) \
                 SELECT abs(\"x1\") AS \"r\" FROM \"input\" ORDER BY \"ordinal\" ASC NULLS FIRST",
            ],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "decimal literals keep their digits, a negated 2^31 is BIGINT, 1e3 is DOUBLE",
            query: "SELECT 2.50 AS d, 0.01 AS e, 2147483648 AS b, -2147483648 AS n, 1e3 AS f FROM input",
            expected_sql: &[
                "CAST(2.50 AS DECIMAL(3, 2)) AS \"d\"",
                "CAST(0.01 AS DECIMAL(2, 2)) AS \"e\"",
                "CAST(-2147483648 AS BIGINT) AS \"n\"",
            ],
            expected_types: &[
                DatabricksType::Decimal {
                    precision: 3,
                    scale: 2,
                },
                DatabricksType::Decimal {
                    precision: 2,
                    scale: 2,
                },
                DatabricksType::BigInt,
                DatabricksType::BigInt,
                DatabricksType::Double,
            ],
        },
        ConvertTestCase {
            description: "integer division is DOUBLE, modulo keeps INT, NULLs last descending",
            query: "SELECT x1 / 2 AS q, x1 % 3 AS m FROM input ORDER BY q DESC",
            expected_sql: &[
                "(CAST(\"x1\" AS DOUBLE) / CAST(2 AS DOUBLE)) AS \"q\"",
                "ORDER BY \"q\" DESC NULLS LAST",
            ],
            expected_types: &[DatabricksType::Double, DatabricksType::Int],
        },
        ConvertTestCase {
            description: "^ is XOR in Databricks, so it becomes xor rather than DuckDB's power",
            query: "SELECT x1 ^ 2 AS r FROM input",
            expected_sql: &["xor(\"x1\", 2) AS \"r\""],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "LIKE ANY is an OR of LIKEs, and a simple CASE compares its operand",
            query: "SELECT s LIKE ANY ('a%', 'b') AS l, CASE x1 WHEN 1 THEN 'one' ELSE 'other' END AS c FROM input",
            expected_sql: &[
                "((\"s\" LIKE 'a%') OR (\"s\" LIKE 'b')) AS \"l\"",
                "CASE WHEN (\"x1\" = 1) THEN 'one' ELSE 'other' END AS \"c\"",
            ],
            expected_types: &[DatabricksType::Boolean, DatabricksType::String],
        },
        ConvertTestCase {
            description: "a literal cast to FLOAT converts only when no rounding happens",
            query: "SELECT CAST(-2.5 AS FLOAT) AS f FROM input",
            expected_sql: &["CAST(CAST(-2.5 AS DECIMAL(2, 1)) AS FLOAT) AS \"f\""],
            expected_types: &[DatabricksType::Float],
        },
        ConvertTestCase {
            description: "LEFT of a zero or negative length is '', where DuckDB counts from the end",
            query: "SELECT left(s, x1) AS l FROM input",
            expected_sql: &["WHEN \"x1\" <= 0 THEN ''"],
            expected_types: &[DatabricksType::String],
        },
        ConvertTestCase {
            description: "TRY_MOD by -1 is 0 and by 0 is NULL, with no try to hide an argument's error",
            query: "SELECT try_mod(x1, x1) AS m, try_add(x1, x1) AS a FROM input",
            expected_sql: &[
                "CASE WHEN \"x1\" = 0 THEN NULL WHEN \"x1\" = -1 THEN \"x1\" * 0 ELSE \"x1\" % \"x1\" END",
                "(CASE WHEN (CAST(\"x1\" AS HUGEINT) + CAST(\"x1\" AS HUGEINT)) BETWEEN -2147483648 AND 2147483647 \
                 THEN CAST((CAST(\"x1\" AS HUGEINT) + CAST(\"x1\" AS HUGEINT)) AS INTEGER) END)",
            ],
            expected_types: &[DatabricksType::Int, DatabricksType::Int],
        },
        ConvertTestCase {
            description: "TRY_DIVIDE by zero is NULL, where DuckDB's DOUBLE division gives NaN",
            query: "SELECT try_divide(x1, 0) AS q FROM input",
            expected_sql: &["CASE WHEN CAST(0 AS DOUBLE) = 0 THEN NULL ELSE"],
            expected_types: &[DatabricksType::Double],
        },
        ConvertTestCase {
            description: "GET is 0-based and NULL for a negative index, where DuckDB counts from the end",
            query: "SELECT get(array(1, 2147483648), x1) AS g, size(array()) AS n FROM input",
            expected_sql: &[
                "CASE WHEN \"x1\" < 0 THEN NULL ELSE [CAST(1 AS BIGINT), CAST(2147483648 AS BIGINT)][CAST(\"x1\" AS BIGINT) + 1] END AS \"g\"",
                "CAST(len([]) AS INTEGER) AS \"n\"",
            ],
            expected_types: &[DatabricksType::BigInt, DatabricksType::Int],
        },
        ConvertTestCase {
            description: "an array result is written as Databricks's text, each element a string",
            query: "SELECT array(x1, NULL) AS a FROM input",
            expected_sql: &[
                "CAST(to_json(list_transform([\"x1\", NULL], lambda element: CAST(element AS VARCHAR))) AS VARCHAR) AS \"a\"",
            ],
            expected_types: &[DatabricksType::Array(Element::Int)],
        },
        ConvertTestCase {
            description: "text without a digit fails an integer cast in both engines, so it converts",
            query: "SELECT CAST('x' AS INT) AS i FROM input",
            expected_sql: &["CAST('x' AS INTEGER) AS \"i\""],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "division by a column raises on zero, where DuckDB's DOUBLE division gives infinity",
            query: "SELECT x1 / x1 AS q, x1 % -3 AS m FROM input",
            expected_sql: &[
                "WHEN CAST(\"x1\" AS DOUBLE) = 0 THEN CAST(0 % CAST(CAST(\"x1\" AS DOUBLE) AS INTEGER) AS DOUBLE)",
                "CASE WHEN -3 = -1 THEN \"x1\" * 0 ELSE \"x1\" % -3 END AS \"m\"",
            ],
            expected_types: &[DatabricksType::Double, DatabricksType::Int],
        },
        ConvertTestCase {
            description: "MOD of a decimal stays an exact decimal, typed as Databricks types it",
            query: "SELECT mod(CAST(x1 AS DECIMAL(12,2)), 3) AS m FROM input",
            expected_sql: &[
                "CAST(CAST(\"x1\" AS DECIMAL(12, 2)) % CAST(3 AS DECIMAL(10, 0)) AS DECIMAL(12, 2))",
            ],
            expected_types: &[DatabricksType::Decimal {
                precision: 12,
                scale: 2,
            }],
        },
        ConvertTestCase {
            description: "DAYOFWEEK counts Sunday as 1, one more than DuckDB's dayofweek",
            query: "SELECT dayofweek(DATE'2016-07-30') AS d FROM input",
            expected_sql: &["CAST(dayofweek(DATE '2016-07-30') + 1 AS INTEGER) AS \"d\""],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "COUNT_IF over only NULLs is 0, where DuckDB gives NULL; SUM of INT is BIGINT",
            query: "SELECT count_if(x1 > 0) AS c, sum(x1) AS t FROM input",
            expected_sql: &[
                "CAST(coalesce(count_if((\"x1\" > 0)), 0) AS BIGINT) AS \"c\"",
                "CAST(sum(\"x1\") AS BIGINT) AS \"t\"",
            ],
            expected_types: &[DatabricksType::BigInt, DatabricksType::BigInt],
        },
        ConvertTestCase {
            description: "DATE_ADD wraps its day count as an INT, as Databricks does",
            query: "SELECT date_add(DATE'2016-07-30', x1) AS d FROM input",
            expected_sql: &["% 4294967296 + 4294967296) % 4294967296 - 2147483648"],
            expected_types: &[DatabricksType::Date],
        },
        ConvertTestCase {
            description: "a left shift wraps through BIGINT with its count masked, where DuckDB raises",
            query: "SELECT x1 << 31 AS l, substr(s, -2) AS t FROM input",
            expected_sql: &[
                "(((CAST(\"x1\" AS BIGINT) & CAST('4294967295' AS BIGINT)) << (CAST(31 AS INTEGER) & 31))",
                "WHEN -2 < 0 THEN length(\"s\") + -2 ELSE 0 END",
            ],
            expected_types: &[DatabricksType::Int, DatabricksType::String],
        },
        ConvertTestCase {
            description: "TYPEOF is the type's lower-case name, and TRY_MAKE_TIMESTAMP checks the hour DuckDB would roll over",
            query: "SELECT typeof(x1) AS t, try_make_timestamp(x1, 1, 1, x1, 0, 0) AS m FROM input",
            expected_sql: &[
                "'int' AS \"t\"",
                "NOT (\"x1\" BETWEEN 0 AND 23 AND 0 BETWEEN 0 AND 59",
            ],
            expected_types: &[DatabricksType::String, DatabricksType::Timestamp],
        },
        ConvertTestCase {
            description: "PMOD wraps its INT sum as Java does, and UNIX_SECONDS rounds before 1970 down",
            query: "SELECT pmod(x1, x1) AS p, unix_seconds(TIMESTAMP'1969-12-31 23:59:59.5') AS u FROM input",
            expected_sql: &[
                "% CAST('4294967296' AS BIGINT) - CAST('2147483648' AS BIGINT))",
                "% 1000000) + 1000000) % 1000000) // 1000000)",
            ],
            expected_types: &[DatabricksType::Int, DatabricksType::BigInt],
        },
        ConvertTestCase {
            description: "DEGREES reads its number as DOUBLE, and a STRING argument to SQRT reads as DOUBLE, raising for digitless text",
            query: "SELECT degrees(x1) AS d, sqrt(s) AS q FROM input",
            expected_sql: &[
                "degrees(CAST(\"x1\" AS DOUBLE)) AS \"d\"",
                "WHEN NOT regexp_matches(\"s\", '(?i)[0-9]|inf|nan') THEN CAST(error('text rejected as a number: the text cast to DOUBLE holds no number') AS DOUBLE)",
            ],
            expected_types: &[DatabricksType::Double, DatabricksType::Double],
        },
        ConvertTestCase {
            description: "a STRING argument to HOUR reads as a timestamp only with an hour below 24, raising where DuckDB cannot read the fields",
            query: "SELECT hour(s) AS h FROM input",
            expected_sql: &[
                "([01][0-9]|2[0-3]):[0-9]{2}:[0-9]{2}",
                "THEN coalesce(TRY_CAST(\"s\" AS TIMESTAMPTZ), CAST(error('invalid timestamp field format: the text is no valid timestamp') AS TIMESTAMPTZ))",
            ],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "a STRING argument to a date field reads as DATE, raising for digitless text as Databricks does",
            query: "SELECT year(s) AS y FROM input",
            expected_sql: &[
                "WHEN regexp_full_match(\"s\", '[1-9][0-9]{3}-[0-9]{2}-[0-9]{2}') THEN CAST(\"s\" AS DATE)",
                "THEN CAST(error('invalid date field format: the text read as a date holds no date') AS DATE)",
            ],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "PMOD of TINYINT adds in INT before narrowing, so pmod(-1Y, -128Y) stays -1",
            query: "SELECT pmod(CAST(x1 AS TINYINT), CAST(x1 AS TINYINT)) AS p FROM input",
            expected_sql: &[
                "AS INTEGER) + CAST(\"x1\" AS TINYINT)) % CAST(\"x1\" AS TINYINT) AS TINYINT) ELSE",
            ],
            expected_types: &[DatabricksType::TinyInt],
        },
        ConvertTestCase {
            description: "a hex literal is a BLOB, whose LENGTH counts bytes and ARRAY_REPEAT of a NULL count is NULL",
            query: "SELECT md5(X'0102') AS m, length(X'0102') AS n, array_repeat(x1, x1) AS a FROM input",
            expected_sql: &[
                "md5(from_hex('0102')) AS \"m\"",
                "CAST(octet_length(from_hex('0102')) AS INTEGER) AS \"n\"",
                "CASE WHEN \"x1\" IS NULL THEN NULL ELSE list_transform(range(",
            ],
            expected_types: &[
                DatabricksType::String,
                DatabricksType::Int,
                DatabricksType::Array(Element::Int),
            ],
        },
        ConvertTestCase {
            description: "a column pattern raises on an escape Databricks rejects; OVERLAY's p - 1 never wraps",
            query: "SELECT s LIKE s AS l, overlay(s PLACING s FROM x1) AS o FROM input",
            expected_sql: &[
                "THEN error('the LIKE pattern has an escape Databricks rejects') ELSE (\"s\" LIKE \"s\" ESCAPE '\\')",
                "ELSE 0 END) + (CAST(\"x1\" AS BIGINT) - 1)) THEN ''",
            ],
            expected_types: &[DatabricksType::Boolean, DatabricksType::String],
        },
        ConvertTestCase {
            description: "a string array is DuckDB's JSON, raising on a control character no recording shows",
            query: "SELECT array(s) AS a FROM input",
            expected_sql: &["regexp_matches(element, '[\\x00-\\x1f]'))) THEN error("],
            expected_types: &[DatabricksType::Array(Element::String)],
        },
        ConvertTestCase {
            description: "an index of 0 raises; SPLIT_PART is empty and TRY_ELEMENT_AT NULL out of bounds",
            query: "SELECT split_part(s, s, x1) AS p, element_at(array(x1), x1) AS e, \
                    try_element_at(array(s), x1) AS t FROM input",
            expected_sql: &[
                "WHEN \"x1\" = 0 THEN error('argument rejected: the index 0 is invalid",
                "coalesce(list_extract(CASE WHEN \"s\" = '' THEN [\"s\"] ELSE string_split(\"s\", \"s\") END, \"x1\"), '')",
                "THEN error('argument rejected: the index is outside the array')",
                "> len([\"s\"]) THEN NULL ELSE list_extract([\"s\"], \"x1\") END",
            ],
            expected_types: &[
                DatabricksType::String,
                DatabricksType::Int,
                DatabricksType::String,
            ],
        },
        ConvertTestCase {
            description: "MONTHS_BETWEEN counts whole seconds over 31-day months, rounded half up to 8 places",
            query: "SELECT months_between(DATE'1997-02-28', DATE'1996-10-30') AS m, \
                    months_between(DATE'1997-02-28', DATE'1996-10-30', false) AS u FROM input",
            expected_sql: &[
                "AS DOUBLE) / 2.6784e6) * 1e8)) + CASE WHEN",
                ">= 0.5 THEN 1 ELSE 0 END) / 1e8)",
                "AS DATE) = last_day(CAST(CAST(",
            ],
            expected_types: &[DatabricksType::Double, DatabricksType::Double],
        },
        ConvertTestCase {
            description: "WIDTH_BUCKET is NULL for BIGINT's largest count and wraps a saturated bucket to BIGINT's smallest",
            query: "SELECT width_bucket(x1, 0, 10, x1) AS w FROM input",
            expected_sql: &[
                "= 9223372036854775807 OR isnan(CAST(\"x1\" AS DOUBLE))",
                ">= 9.223372036854775808e18 THEN CAST(-9223372036854775808 AS BIGINT)",
            ],
            expected_types: &[DatabricksType::BigInt],
        },
        ConvertTestCase {
            description: "ILIKE lowers both sides, raising on a character whose case no rule folds",
            query: "SELECT s ILIKE 'A\\\\%' AS c, s NOT ILIKE ANY (s, 'b_') AS a FROM input",
            expected_sql: &[
                "THEN error('the text holds a character whose case no rule maps as Databricks does') ELSE lower(\"s\") END)",
                "LIKE (CASE WHEN regexp_matches('A\\%', '[^\\x{0}-\\x{12f}\\x{131}-",
                "ESCAPE '\\')",
                "THEN error('the LIKE pattern has an escape Databricks rejects')",
            ],
            expected_types: &[DatabricksType::Boolean, DatabricksType::Boolean],
        },
        ConvertTestCase {
            description: "UPPER and LCASE map case, raising on a character DuckDB was not seen to map as Java does",
            query: "SELECT upper(s) AS u, lcase(s) AS l FROM input",
            expected_sql: &[
                "ELSE upper(\"s\") END) AS \"u\"",
                "ELSE lower(\"s\") END) AS \"l\"",
                "[^\\x{0}-\\x{de}\\x{e0}-\\x{148}\\x{14a}-",
            ],
            expected_types: &[DatabricksType::String, DatabricksType::String],
        },
        ConvertTestCase {
            description: "RLIKE of text finds it; a column pattern raises for a metacharacter",
            query: "SELECT rlike(s, 'a,b') AS t, s RLIKE s AS c FROM input",
            expected_sql: &[
                "contains(\"s\", 'a,b') AS \"t\"",
                "WHEN regexp_matches(\"s\", '[\\\\.^$|?*+()\\[\\]{}]') THEN error(",
            ],
            expected_types: &[DatabricksType::Boolean, DatabricksType::Boolean],
        },
        ConvertTestCase {
            description: "REGEXP_REPLACE writes Java's . without its line terminators and $1 as \\1",
            query: "SELECT regexp_replace(s, 'a.(b)', '[$1]') AS r, regexp_like(s, '[\\\\s]x$') AS e FROM input",
            expected_sql: &[
                "regexp_replace(\"s\", 'a[^\\n\\r\\x{85}\\x{2028}\\x{2029}](b)', '[\\1]', 'g') AS \"r\"",
                "WHEN suffix(\"s\", chr(13) || chr(10)) THEN error(",
                "regexp_matches(\"s\", '[ \\t\\n\\x0B\\f\\r]x(?:\\r\\n|[\\n\\r\\x{85}\\x{2028}\\x{2029}])?$')",
            ],
            expected_types: &[DatabricksType::String, DatabricksType::Boolean],
        },
        ConvertTestCase {
            description: "REGEXP_EXTRACT raises for a match with too few groups; EXTRACT_ALL gives '' for an idle group",
            query: "SELECT regexp_extract(s, 'a+') AS e, regexp_extract_all(s, '(a)|b', 1) AS l FROM input",
            expected_sql: &[
                "WHEN regexp_matches(\"s\", 'a+') THEN error('argument rejected: the group index is outside the pattern''s groups')",
                "list_transform(regexp_extract_all(\"s\", '(a)|b', 1), lambda element: coalesce(element, ''))",
            ],
            expected_types: &[
                DatabricksType::String,
                DatabricksType::Array(Element::String),
            ],
        },
        ConvertTestCase {
            description: "REGEXP_COUNT of '' and REGEXP_INSTR count UTF-16 units as Java does",
            query: "SELECT regexp_count(s, '') AS c, regexp_instr(s, '[0-9]+') AS i FROM input",
            expected_sql: &[
                "CAST(length(\"s\") + length(regexp_replace(\"s\", '[^\\x{10000}-\\x{10FFFF}]', '', 'g')) + 1 AS INTEGER) AS \"c\"",
                "regexp_extract(\"s\", '^((?s:.*?))(?:[0-9]+)', 1)",
            ],
            expected_types: &[DatabricksType::Int, DatabricksType::Int],
        },
        ConvertTestCase {
            description: "CONV reads digits up to the first invalid one in unsigned 64 bits, NULL for a base outside 2 to 36",
            query: "SELECT conv(s, 16, x1) AS r FROM input",
            expected_sql: &[
                "WHEN 16 NOT BETWEEN 2 AND 36 OR (\"x1\" NOT BETWEEN 2 AND 36 AND \"x1\" NOT BETWEEN -36 AND -2) THEN NULL",
                "AS UBIGINT) AS HUGEINT)",
            ],
            expected_types: &[DatabricksType::String],
        },
        ConvertTestCase {
            description: "DATE_FORMAT writes Java's pattern letters and quoted text with strftime",
            query: "SELECT date_format(CAST(s AS DATE), 'MMM d, yyyy ''at'' h a') AS r FROM input",
            expected_sql: &[
                "THEN strftime((CASE WHEN \"s\" IS NULL",
                "'%b %-d, %Y at %-I %p')",
            ],
            expected_types: &[DatabricksType::String],
        },
        ConvertTestCase {
            description: "SHA2 of a column length: SHA-256 for 256 or 0, raising for a length DuckDB lacks, else NULL",
            query: "SELECT sha2(s, x1) AS h FROM input",
            expected_sql: &[
                "WHEN \"x1\" IN (256, 0) THEN sha256(\"s\")",
                "WHEN \"x1\" IN (224, 384, 512) THEN error('the SHA2 digest length has no DuckDB function') END",
            ],
            expected_types: &[DatabricksType::String],
        },
        ConvertTestCase {
            description: "MAKE_DATE and MAKE_TIMESTAMP raise for a field out of bounds or a day the month lacks",
            query: "SELECT make_date(x1, x1, x1) AS d, make_timestamp(x1, x1, x1, x1, x1, x1) AS t FROM input",
            expected_sql: &[
                "abs(CAST(\"x1\" AS BIGINT)) > 999999999 THEN error('argument rejected: a date or time field is out of bounds')",
                "THEN coalesce(try(make_date(CAST(\"x1\" AS BIGINT)",
                "THEN coalesce(try(make_timestamp(CAST(\"x1\" AS BIGINT)",
            ],
            expected_types: &[DatabricksType::Date, DatabricksType::Timestamp],
        },
        ConvertTestCase {
            description: "ELT picks an argument as a string, raising for an index outside them",
            query: "SELECT elt(x1, x1, s) AS e FROM input",
            expected_sql: &[
                "WHEN \"x1\" < 1 OR \"x1\" > 2 THEN error('argument rejected: the index is outside the array')",
                "WHEN \"x1\" = 1 THEN CAST(\"x1\" AS VARCHAR) WHEN \"x1\" = 2 THEN \"s\" END",
            ],
            expected_types: &[DatabricksType::String],
        },
        ConvertTestCase {
            description: "a string compared with an INT reads as BIGINT, raising for text that is not signed digits",
            query: "SELECT s = x1 AS e, x1 BETWEEN s AND 3 AS b FROM input",
            expected_sql: &[
                "(CASE WHEN \"s\" IS NULL THEN CAST(NULL AS BIGINT) \
                 WHEN regexp_full_match(\"s\", ' *[+-]?[0-9]{1,18} *') THEN CAST(trim(\"s\", ' ') AS BIGINT)",
                "OR regexp_full_match(\"s\", ' *[+-]?([0-9]+\\.[0-9]+|0*[1-9][0-9]{19,}) *')",
                "THEN CAST(error('text rejected as a number: the text compared with an integer is not a number') AS BIGINT)",
                "= CAST(\"x1\" AS BIGINT)",
            ],
            expected_types: &[DatabricksType::Boolean, DatabricksType::Boolean],
        },
        ConvertTestCase {
            description: "BIT_GET raises outside the width; NEXT_DAY reads weekday names without case",
            query: "SELECT bit_get(x1, x1) AS b, next_day(DATE'2016-07-30', s) AS n FROM input",
            expected_sql: &[
                "WHEN \"x1\" < 0 OR \"x1\" >= 32 THEN CAST(error('argument rejected: the bit position is outside the value') AS TINYINT)",
                "ELSE CAST((\"x1\" >> CAST(\"x1\" AS INTEGER)) & 1 AS TINYINT) END",
                "WHEN 'SU' THEN 7 WHEN 'SUN' THEN 7 WHEN 'SUNDAY' THEN 7 END",
                "THEN CAST(error('argument rejected: the text names no day of the week') AS DATE)",
            ],
            expected_types: &[DatabricksType::TinyInt, DatabricksType::Date],
        },
        ConvertTestCase {
            description: "ARRAY_JOIN of integers by a column delimiter; FROM_UNIXTIME writes Databricks's default format",
            query: "SELECT array_join(array(x1), s) AS j, from_unixtime(x1) AS u FROM input",
            expected_sql: &[
                "ELSE substr(list_reduce(list_prepend('', list_filter(list_transform([\"x1\"], lambda element: CAST(element AS VARCHAR))",
                "lambda joined, element: joined || \"s\" || element), length(\"s\") + 1) END",
                "THEN strftime(make_timestamp(CAST(\"x1\" AS BIGINT) * 1000000), '%Y-%m-%d %H:%M:%S')",
                "ELSE CAST(error('the year of the time is outside those no rule shows Databricks writing') AS VARCHAR) END",
            ],
            expected_types: &[DatabricksType::String, DatabricksType::String],
        },
        ConvertTestCase {
            description: "AVG of integers is the exact sum over the count, raising past 2^53; MEDIAN of INT interpolates exactly",
            query: "SELECT avg(x1) AS a, median(x1) AS m FROM input",
            expected_sql: &[
                "WHEN sum(abs(CAST(\"x1\" AS HUGEINT))) >= 9007199254740992",
                "ELSE CAST(sum(CAST(\"x1\" AS BIGINT)) AS DOUBLE) / CAST(count(\"x1\") AS DOUBLE) END",
                "CAST(quantile_cont(CAST(\"x1\" AS DOUBLE), 0.5) AS DOUBLE)",
            ],
            expected_types: &[DatabricksType::Double, DatabricksType::Double],
        },
        ConvertTestCase {
            description: "trim is told to trim only spaces, as Databricks does",
            query: "SELECT trim(s) AS t FROM input",
            expected_sql: &["trim(\"s\", ' ') AS \"t\""],
            expected_types: &[DatabricksType::String],
        },
        ConvertTestCase {
            description: "NULLIF keeps its first argument's type, comparing text with an integer as BIGINT",
            query: "SELECT nullif(s, x1) AS t, nullif(x1, CAST(x1 AS BIGINT)) AS n FROM input",
            expected_sql: &["ELSE \"s\" END) AS \"t\""],
            expected_types: &[DatabricksType::String, DatabricksType::Int],
        },
        ConvertTestCase {
            description: "a BIGINT shift count narrows to INT, as Databricks's implicit cast does",
            query: "SELECT shiftleft(x1, CAST(x1 AS BIGINT)) AS r FROM input",
            expected_sql: &["CAST(CAST(\"x1\" AS BIGINT) AS INTEGER)"],
            expected_types: &[DatabricksType::Int],
        },
        ConvertTestCase {
            description: "text read as a DATE for MONTH and as a BOOLEAN for BOOLEAN, by Databricks's words",
            query: "SELECT month(s) AS m, boolean(s) AS b FROM input",
            expected_sql: &["lower(\"s\") IN ('t', 'true', 'y', 'yes', '1') THEN TRUE"],
            expected_types: &[DatabricksType::Int, DatabricksType::Boolean],
        },
        ConvertTestCase {
            description: "REVERSE of a string reverses code points, not DuckDB's grapheme clusters",
            query: "SELECT reverse(s) AS r FROM input",
            expected_sql: &["list_reverse(regexp_extract_all(\"s\", '(?s).'))"],
            expected_types: &[DatabricksType::String],
        },
        ConvertTestCase {
            description: "ZEROIFNULL widens a decimal to hold an INT 0, PMOD keeps the remainder's type",
            query: "SELECT zeroifnull(CAST(x1 AS DECIMAL(12,2))) AS z, pmod(CAST(x1 AS DECIMAL(12,2)), 7) AS p FROM input",
            expected_sql: &["< 0 THEN CAST((CAST("],
            expected_types: &[
                DatabricksType::Decimal {
                    precision: 12,
                    scale: 2,
                },
                DatabricksType::Decimal {
                    precision: 12,
                    scale: 2,
                },
            ],
        },
    ];
    for test_case in test_cases {
        let converted = analyze(&format!("{FIXTURE}{}", test_case.query))
            .unwrap_or_else(|refusal| panic!("{}: {}", test_case.description, refusal.reason));
        for expected in test_case.expected_sql {
            assert!(
                converted.sql.contains(expected),
                "{}: {}",
                test_case.description,
                converted.sql
            );
        }
        let types: Vec<DatabricksType> = converted
            .columns
            .iter()
            .map(|(_, data_type)| *data_type)
            .collect();
        assert_eq!(types, test_case.expected_types, "{}", test_case.description);
    }
}

#[test]
fn given_a_hazard_or_unknown_construct_when_converted_then_it_is_refused() {
    let test_cases = [
        RefuseTestCase {
            description: "a date field of an integer, which Databricks would read as text",
            query: "SELECT dayofweek(x1) AS r FROM input",
            expected_reason: "DAYOFWEEK of (INT)",
        },
        RefuseTestCase {
            description: "a regular expression with an inline flag, whose Java meaning no rule shows RE2 sharing",
            query: "SELECT regexp_like(s, '(?i)ab') AS r FROM input",
            expected_reason: "REGEXP_LIKE",
        },
        RefuseTestCase {
            description: "a replacement after a pattern that can match empty, where Java and RE2 step on differently",
            query: "SELECT regexp_replace(s, 'a*', '-') AS r FROM input",
            expected_reason: "REGEXP_REPLACE",
        },
        RefuseTestCase {
            description: "a word boundary, which Java and RE2 draw differently",
            query: "SELECT regexp_substr(s, '\\\\bab') AS r FROM input",
            expected_reason: "REGEXP_SUBSTR",
        },
        RefuseTestCase {
            description: "a week-based year, which Spark rejects and strftime would write",
            query: "SELECT date_format(CAST(s AS DATE), 'YYYY') AS r FROM input",
            expected_reason: "DATE_FORMAT",
        },
        RefuseTestCase {
            description: "a DOUBLE number for CONV, whose text Java writes differently",
            query: "SELECT conv(CAST(x1 AS DOUBLE), 10, 2) AS r FROM input",
            expected_reason: "CONV",
        },
        RefuseTestCase {
            description: "title-casing, whose word breaks no rule shows DuckDB sharing",
            query: "SELECT initcap(s) AS r FROM input",
            expected_reason: "INITCAP",
        },
        RefuseTestCase {
            description: "a decimal quotient, which Databricks keeps exact and DOUBLE would round",
            query: "SELECT CAST(x1 AS DECIMAL(12,2)) / x1 AS r FROM input",
            expected_reason: "DECIMAL(12,2) div INT",
        },
        RefuseTestCase {
            description: "a LIKE pattern holding Databricks's escape",
            query: "SELECT s LIKE 'a\\\\b' AS r FROM input",
            expected_reason: "backslash",
        },
        RefuseTestCase {
            description: "an unaliased call, whose name Databricks generates",
            query: "SELECT abs(x1) FROM input",
            expected_reason: "unaliased",
        },
        RefuseTestCase {
            description: "a column spelled in another case, which names the result differently",
            query: "SELECT X1 FROM input",
            expected_reason: "spelled unlike",
        },
        RefuseTestCase {
            description: "a NULL result column, VOID in Databricks",
            query: "SELECT NULL AS r FROM input",
            expected_reason: "VOID result column",
        },
        RefuseTestCase {
            description: "ORDER BY a position",
            query: "SELECT x1 AS r FROM input ORDER BY 1",
            expected_reason: "position",
        },
        RefuseTestCase {
            description: "a string compared with a DOUBLE, whose reading of the text no rule shows",
            query: "SELECT s = CAST(x1 AS DOUBLE) AS r FROM input",
            expected_reason: "compares",
        },
        RefuseTestCase {
            description: "0.1 is no binary fraction, so engines may round its FLOAT differently",
            query: "SELECT CAST(0.1 AS FLOAT) AS f FROM input",
            expected_reason: "FLOAT",
        },
        RefuseTestCase {
            description: "seconds finer than a microsecond, which Databricks's exact conversion rejects",
            query: "SELECT timestamp_seconds(CAST(x1 AS DECIMAL(17,7))) AS r FROM input",
            expected_reason: "TIMESTAMP_SECONDS",
        },
        RefuseTestCase {
            description: "SHA2 of a length DuckDB has no digest for",
            query: "SELECT sha2(s, 384) AS h FROM input",
            expected_reason: "SHA2",
        },
        RefuseTestCase {
            description: "text DuckDB reads as an infinite date, where Databricks raises",
            query: "SELECT CAST('inf' AS DATE) AS d FROM input",
            expected_reason: "DATE",
        },
        RefuseTestCase {
            description: "LEVENSHTEIN, which DuckDB counts in bytes and Databricks in characters",
            query: "SELECT levenshtein(s, s) AS r FROM input",
            expected_reason: "LEVENSHTEIN",
        },
        RefuseTestCase {
            description: "TRY_MAKE_TIMESTAMP over an expression, whose own error DuckDB's try would hide",
            query: "SELECT try_make_timestamp(x1 + 1, 1, 1, 0, 0, 0) AS m FROM input",
            expected_reason: "MAKE_TIMESTAMP",
        },
        RefuseTestCase {
            description: "MAX of arrays, whose NULL elements DuckDB orders above values",
            query: "SELECT max(array(x1)) AS m FROM input",
            expected_reason: "MAX",
        },
        RefuseTestCase {
            description: "GREATEST of arrays, whose NULL elements DuckDB orders above values",
            query: "SELECT greatest(array(x1), array(1)) AS g FROM input",
            expected_reason: "GREATEST of arrays",
        },
        RefuseTestCase {
            description: "an exponent literal beyond DOUBLE, which Databricks rejects",
            query: "SELECT 1e400 AS f FROM input",
            expected_reason: "1e400",
        },
        RefuseTestCase {
            description: "an exponent literal just above DOUBLE's largest, which rounds to it but Databricks rejects",
            query: "SELECT 1.7976931348623158e308 AS f FROM input",
            expected_reason: "1.7976931348623158e308",
        },
        RefuseTestCase {
            description: "SIN, whose last bit Databricks rounds unlike DuckDB's libm",
            query: "SELECT sin(x1) AS r FROM input",
            expected_reason: "SIN",
        },
        RefuseTestCase {
            description: "EXP, whose last bit Databricks's StrictMath rounds unlike DuckDB",
            query: "SELECT exp(x1) AS r FROM input",
            expected_reason: "EXP",
        },
        RefuseTestCase {
            description: "MOD of text and a decimal, which Databricks reads as DOUBLE, not BIGINT",
            query: "SELECT mod(s, 2.5) AS r FROM input",
            expected_reason: "MOD",
        },
        RefuseTestCase {
            description: "an OFFSET no rule reads",
            query: "SELECT x1 AS r FROM input LIMIT 1 OFFSET 1",
            expected_reason: "offset",
        },
    ];
    for test_case in test_cases {
        let reason = analyze(&format!("{FIXTURE}{}", test_case.query))
            .expect_err(test_case.description)
            .reason;
        assert!(
            reason.contains(test_case.expected_reason),
            "{}: {reason}",
            test_case.description
        );
    }
}

#[test]
fn given_arguments_outside_a_rule_when_converted_then_the_refusal_is_a_type_condition() {
    let test_cases = [
        CategoryTestCase {
            description: "an array where a string function takes text, which Databricks rejects at compile time",
            query: "SELECT upper(array(x1)) AS r FROM input",
            expected_category: RefusalCategory::TypeCondition,
        },
        CategoryTestCase {
            description: "a function no rule converts at all",
            query: "SELECT sin(x1) AS r FROM input",
            expected_category: RefusalCategory::MissingRule,
        },
    ];
    for test_case in test_cases {
        let category = analyze(&format!("{FIXTURE}{}", test_case.query))
            .expect_err(test_case.description)
            .category;
        assert_eq!(
            category, test_case.expected_category,
            "{}",
            test_case.description
        );
    }
}
