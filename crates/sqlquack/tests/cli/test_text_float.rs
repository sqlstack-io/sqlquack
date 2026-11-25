use crate::helpers::{converted_first_value, converted_run_error, parsed_copies};
use crate::test_types::{ChainValueTestCase, PatternCopiesTestCase, RunErrorTestCase};

#[test]
fn given_text_cast_to_float_when_converting_then_duckdb_returns_snowflakes_value() {
    let test_cases = [
        ChainValueTestCase {
            description: "CAST: an integer",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '1' AS a)",
            expected_value: "1",
        },
        ChainValueTestCase {
            description: "CAST: a decimal between spaces",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT ' 2.5 ' AS a)",
            expected_value: "2.5",
        },
        ChainValueTestCase {
            description: "CAST: an exponent between a tab and a line feed",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '\t-1e3\n' AS a)",
            expected_value: "-1000",
        },
        ChainValueTestCase {
            description: "CAST: a decimal without a whole part",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '.5' AS a)",
            expected_value: "0.5",
        },
        ChainValueTestCase {
            description: "CAST: a decimal point without a fraction",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '5.' AS a)",
            expected_value: "5",
        },
        ChainValueTestCase {
            description: "CAST: NaN",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT 'nan' AS a)",
            expected_value: "NaN",
        },
        ChainValueTestCase {
            description: "CAST: a signed NaN in another case",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '-NaN' AS a)",
            expected_value: "NaN",
        },
        ChainValueTestCase {
            description: "CAST: infinity",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT 'inf' AS a)",
            expected_value: "inf",
        },
        ChainValueTestCase {
            description: "CAST: negative infinity spelled out",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '-Infinity' AS a)",
            expected_value: "-inf",
        },
        ChainValueTestCase {
            description: "CAST: signed infinity",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '+inf' AS a)",
            expected_value: "inf",
        },
        ChainValueTestCase {
            description: "CAST: a number beyond the double range",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '1e400' AS a)",
            expected_value: "inf",
        },
        ChainValueTestCase {
            description: "TRY_CAST: an integer",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '1' AS a)",
            expected_value: "1",
        },
        ChainValueTestCase {
            description: "TRY_CAST: a decimal between spaces",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT ' 2.5 ' AS a)",
            expected_value: "2.5",
        },
        ChainValueTestCase {
            description: "TRY_CAST: an exponent between a tab and a line feed",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '\t-1e3\n' AS a)",
            expected_value: "-1000",
        },
        ChainValueTestCase {
            description: "TRY_CAST: a decimal without a whole part",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '.5' AS a)",
            expected_value: "0.5",
        },
        ChainValueTestCase {
            description: "TRY_CAST: a decimal point without a fraction",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '5.' AS a)",
            expected_value: "5",
        },
        ChainValueTestCase {
            description: "TRY_CAST: NaN",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT 'nan' AS a)",
            expected_value: "NaN",
        },
        ChainValueTestCase {
            description: "TRY_CAST: a signed NaN in another case",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '-NaN' AS a)",
            expected_value: "NaN",
        },
        ChainValueTestCase {
            description: "TRY_CAST: infinity",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT 'inf' AS a)",
            expected_value: "inf",
        },
        ChainValueTestCase {
            description: "TRY_CAST: negative infinity spelled out",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '-Infinity' AS a)",
            expected_value: "-inf",
        },
        ChainValueTestCase {
            description: "TRY_CAST: signed infinity",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '+inf' AS a)",
            expected_value: "inf",
        },
        ChainValueTestCase {
            description: "TRY_CAST: a word",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT 'abc' AS a)",
            expected_value: "none",
        },
        ChainValueTestCase {
            description: "TRY_CAST: digits with an underscore",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '1_000' AS a)",
            expected_value: "none",
        },
        ChainValueTestCase {
            description: "TRY_CAST: hexadecimal",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '0x10' AS a)",
            expected_value: "none",
        },
        ChainValueTestCase {
            description: "TRY_CAST: a number beyond the double range",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '1e400' AS a)",
            expected_value: "none",
        },
        ChainValueTestCase {
            description: "TRY_CAST: empty text",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '' AS a)",
            expected_value: "none",
        },
        ChainValueTestCase {
            description: "TRY_CAST: an Arabic-Indic digit",
            sql: "SELECT COALESCE(TO_VARCHAR(TRY_CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '\u{661}' AS a)",
            expected_value: "none",
        },
    ];
    for test_case in test_cases {
        let value = converted_first_value(test_case.sql);

        assert_eq!(value, test_case.expected_value, "{}", test_case.description);
    }
}

#[test]
fn given_unrecognized_text_cast_to_float_when_running_then_duckdb_raises_snowflakes_error() {
    let test_cases = [
        RunErrorTestCase {
            description: "CAST: a word",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT 'abc' AS a)",
            expected_mention: "Numeric value is not recognized",
        },
        RunErrorTestCase {
            description: "CAST: digits with an underscore",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '1_000' AS a)",
            expected_mention: "Numeric value is not recognized",
        },
        RunErrorTestCase {
            description: "CAST: hexadecimal",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '0x10' AS a)",
            expected_mention: "Numeric value is not recognized",
        },
        RunErrorTestCase {
            description: "CAST: empty text",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '' AS a)",
            expected_mention: "Numeric value is not recognized",
        },
        RunErrorTestCase {
            description: "CAST: an Arabic-Indic digit",
            sql: "SELECT COALESCE(TO_VARCHAR(CAST(a AS FLOAT)), 'none') AS c FROM (SELECT '\u{661}' AS a)",
            expected_mention: "Numeric value is not recognized",
        },
    ];
    for test_case in test_cases {
        let error = converted_run_error(test_case.sql);

        assert!(
            error
                .as_deref()
                .is_some_and(|error| error.contains(test_case.expected_mention)),
            "{}: {error:?}",
            test_case.description
        );
    }
}

#[test]
fn given_text_cast_to_float_when_converting_then_duckdb_binds_no_chr_call() {
    let test_cases = [
        PatternCopiesTestCase {
            description: "CAST trims with one whitespace literal",
            sql: "SELECT CAST(a AS FLOAT) AS c FROM (SELECT '1' AS a)",
            pattern: "\"function_name\":\"chr\"",
            expected_copies: 0,
        },
        PatternCopiesTestCase {
            description: "TRY_CAST trims with one whitespace literal",
            sql: "SELECT TRY_CAST(a AS FLOAT) AS c FROM (SELECT '1' AS a)",
            pattern: "\"function_name\":\"chr\"",
            expected_copies: 0,
        },
    ];
    for test_case in test_cases {
        let copies = parsed_copies(test_case.sql, test_case.pattern);

        assert_eq!(
            copies, test_case.expected_copies,
            "{}",
            test_case.description
        );
    }
}
