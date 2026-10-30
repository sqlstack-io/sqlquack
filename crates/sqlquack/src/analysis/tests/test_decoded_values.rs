use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::FirstValueTestCase;

#[test]
fn given_decoded_or_parsed_values_when_converting_then_they_read_as_recorded() {
    let test_cases = [
        FirstValueTestCase {
            description: "decoded bytes written by HEX_ENCODE, line feeds ignored",
            sql: "SELECT HEX_ENCODE(TRY_BASE64_DECODE_BINARY('b3Jk' || CHR(10) || 'ZXI=')) AS v",
            expected_outcome: Ok("6F72646572"),
        },
        FirstValueTestCase {
            description: "a line feed inside a group of four is ignored, as recorded",
            sql: "SELECT HEX_ENCODE(TRY_BASE64_DECODE_BINARY('b3J' || CHR(10) || 'kZXI=')) AS v",
            expected_outcome: Ok("6F72646572"),
        },
        FirstValueTestCase {
            description: "two line feeds in a row are ignored, as recorded",
            sql: "SELECT HEX_ENCODE(TRY_BASE64_DECODE_BINARY('b3Jk' || CHR(10) || CHR(10) || \
                  'ZXI=')) AS v",
            expected_outcome: Ok("6F72646572"),
        },
        FirstValueTestCase {
            description: "a carriage return before a line feed between groups is NULL, as recorded",
            sql: "SELECT COALESCE(HEX_ENCODE(TRY_BASE64_DECODE_BINARY('b3JkZXIg' || CHR(13) || \
                  CHR(10) || 'b3JkZXI=')), 'null') AS v",
            expected_outcome: Ok("null"),
        },
        FirstValueTestCase {
            description: "padding bits left set still decode",
            sql: "SELECT HEX_ENCODE(TRY_BASE64_DECODE_BINARY('YR=='), 0) AS v",
            expected_outcome: Ok("61"),
        },
        FirstValueTestCase {
            description: "missing padding is NULL",
            sql: "SELECT COALESCE(HEX_ENCODE(TRY_BASE64_DECODE_BINARY('YQ')), 'null') AS v",
            expected_outcome: Ok("null"),
        },
        FirstValueTestCase {
            description: "SUBSTR of decoded bytes counts bytes, a negative start from the end",
            sql: "SELECT HEX_ENCODE(SUBSTR(TRY_BASE64_DECODE_BINARY('YWJj'), -2, 2)) AS v",
            expected_outcome: Ok("6263"),
        },
        FirstValueTestCase {
            description: "decoded bytes read as UTF-8 text",
            sql: "SELECT TO_VARCHAR(SUBSTRING(TRY_BASE64_DECODE_BINARY('R3LDtsOfZQ=='), 1, 2), \
                  'UTF-8') AS v",
            expected_outcome: Ok("Gr"),
        },
        FirstValueTestCase {
            description: "bytes that are not UTF-8 read as text raise 100146",
            sql: "SELECT TO_VARCHAR(TRY_BASE64_DECODE_BINARY('/w=='), 'UTF-8') AS v",
            expected_outcome: Err("100146"),
        },
        FirstValueTestCase {
            description: "decoded bytes as a result column are refused",
            sql: "SELECT TRY_BASE64_DECODE_BINARY('YQ==') AS v",
            expected_outcome: Err("decoded bytes other than"),
        },
        FirstValueTestCase {
            description: "DD MON YYYY reads a full month name, any case, without spaces",
            sql: "SELECT TRY_TO_DATE('05MARCH2024', 'DD MON YYYY')::VARCHAR AS v",
            expected_outcome: Ok("2024-03-05"),
        },
        FirstValueTestCase {
            description: "MMMM DD YYYY reads a two-digit year as it is written",
            sql: "SELECT TRY_TO_DATE('sep 5 24', 'MMMM DD YYYY')::VARCHAR AS v",
            expected_outcome: Ok("0024-09-05"),
        },
        FirstValueTestCase {
            description: "an unrecorded spelling of a month-name date raises",
            sql: "SELECT TRY_TO_DATE('05.Mar.2024', 'DD MON YYYY') AS v",
            expected_outcome: Err("date_text_spelling"),
        },
        FirstValueTestCase {
            description: "a month name spelled with a non-ASCII letter Unicode folds raises",
            sql: "SELECT TRY_TO_DATE('05 Augu\u{17f}t 2024', 'DD MON YYYY') AS v",
            expected_outcome: Err("date_text_spelling"),
        },
        FirstValueTestCase {
            description: "spaces on one side of the month only read the date, as recorded",
            sql: "SELECT TRY_TO_DATE('05Mar 2024', 'DD MON YYYY') AS v",
            expected_outcome: Ok("2024-03-05"),
        },
        FirstValueTestCase {
            description: "a three-digit year is read as written, as recorded",
            sql: "SELECT TRY_TO_DATE('05 Mar 202', 'DD MON YYYY') AS v",
            expected_outcome: Ok("0202-03-05"),
        },
        FirstValueTestCase {
            description: "spaced hyphens, unrecorded, raise",
            sql: "SELECT TRY_TO_DATE('05 - Mar - 2024', 'DD MON YYYY') AS v",
            expected_outcome: Err("date_text_spelling"),
        },
        FirstValueTestCase {
            description: "mixed separators are NULL, as recorded",
            sql: "SELECT COALESCE(TRY_TO_DATE('05-Mar/2024', 'DD MON YYYY')::VARCHAR, 'null') AS v",
            expected_outcome: Ok("null"),
        },
        FirstValueTestCase {
            description: "REGEXP_LIKE with i folds the long s onto s, as recorded",
            sql: "SELECT REGEXP_LIKE('\u{17f}', 's', 'i') AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "REGEXP_LIKE with i folds the Kelvin sign onto k, as recorded",
            sql: "SELECT REGEXP_LIKE('\u{212a}', 'k', 'i') AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "REGEXP_LIKE with i does not fold the Kelvin sign onto s, as recorded",
            sql: "SELECT REGEXP_LIKE('\u{212a}', 's', 'i') AS v",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "ILIKE folds the long s onto s, as recorded",
            sql: "SELECT '\u{17f}' ILIKE 's' AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "ILIKE folds the Kelvin sign onto k, as recorded",
            sql: "SELECT '\u{212a}' ILIKE 'k' AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "REGEXP_SUBSTR with i finds the Kelvin sign in a-z, as recorded",
            sql: "SELECT REGEXP_SUBSTR('\u{212a}', '[a-z]', 1, 1, 'i') AS v",
            expected_outcome: Ok("\u{212a}"),
        },
        FirstValueTestCase {
            description: "REGEXP_SUBSTR with i finds the long s in a-z, as recorded",
            sql: "SELECT REGEXP_SUBSTR('\u{17f}', '[a-z]', 1, 1, 'i') AS v",
            expected_outcome: Ok("\u{17f}"),
        },
        FirstValueTestCase {
            description: "ILIKE of text holding a final sigma, whose folding is unrecorded, raises",
            sql: "SELECT '\u{3c2}' ILIKE '\u{3c3}' AS v",
            expected_outcome: Err("case_folding"),
        },
        FirstValueTestCase {
            description: "an ILIKE pattern holding a final sigma is refused",
            sql: "SELECT 'a' ILIKE '\u{3c2}' AS v",
            expected_outcome: Err("no recording shows"),
        },
        FirstValueTestCase {
            description: "an ILIKE pattern holding the long s with an ESCAPE is refused",
            sql: "SELECT 'sabc' ILIKE '\u{17f}%' ESCAPE 's' AS v",
            expected_outcome: Err("folding or lower-casing"),
        },
        FirstValueTestCase {
            description: "a non-ASCII ILIKE pattern with an upper-case letter ESCAPE is refused",
            sql: "SELECT 'S%\u{e9}' ILIKE 'S%\u{e9}' ESCAPE 'S' AS v",
            expected_outcome: Err("folding or lower-casing"),
        },
        FirstValueTestCase {
            description: "ILIKE still folds recorded non-ASCII letters",
            sql: "SELECT '\u{c9}t\u{17f}' ILIKE '\u{e9}TS' AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "four leading spaces, longer than recorded, raise",
            sql: "SELECT TRY_TO_DATE('    05 Mar 2024', 'DD MON YYYY') AS v",
            expected_outcome: Err("date_text_spelling"),
        },
        FirstValueTestCase {
            description: "doubled spaces on both sides of the month read the date, as recorded",
            sql: "SELECT TRY_TO_DATE('05  Mar  2024', 'DD MON YYYY') AS v",
            expected_outcome: Ok("2024-03-05"),
        },
        FirstValueTestCase {
            description: "four spaces between MMMM fields, longer than recorded, raise",
            sql: "SELECT TRY_TO_DATE('March    05 2024', 'MMMM DD YYYY') AS v",
            expected_outcome: Err("date_text_spelling"),
        },
        FirstValueTestCase {
            description: "a doubled space on one side of the month reads, as recorded",
            sql: "SELECT TRY_TO_DATE('05  Mar 2024', 'DD MON YYYY') AS v",
            expected_outcome: Ok("2024-03-05"),
        },
        FirstValueTestCase {
            description: "an infinity word with a dotted capital I is not read, as recorded",
            sql: "SELECT COALESCE(TRY_TO_DOUBLE('\u{130}nf')::VARCHAR, 'null') AS v",
            expected_outcome: Ok("null"),
        },
        FirstValueTestCase {
            description: "a month-name format in another case is refused",
            sql: "SELECT TRY_TO_DATE('05 Mar 2024', 'dd mon yyyy') AS v",
            expected_outcome: Err("no recording shows"),
        },
        FirstValueTestCase {
            description: "a text field of an object holding a FLOAT casts to DATE",
            sql: "SELECT OBJECT_CONSTRUCT('d', '2024-02-29', 'f', 1.5::FLOAT):d::DATE::VARCHAR AS v",
            expected_outcome: Ok("2024-02-29"),
        },
        FirstValueTestCase {
            description: "a FLOAT field of such an object cast to DATE raises",
            sql: "SELECT OBJECT_CONSTRUCT('f', 1.5::FLOAT):f::DATE AS v",
            expected_outcome: Err("json_fidelity"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_first_outcome(test_case.sql, test_case.expected_outcome),
            test_case
                .expected_outcome
                .map(str::to_owned)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
