use crate::analysis::tests::helpers::{argument_copies, shop_first_outcome};
use crate::analysis::tests::test_types::{ArgumentCopiesTestCase, FirstValueTestCase};

#[test]
fn given_long_arguments_when_converting_then_each_is_written_once_with_the_same_values() {
    let test_cases = [
        FirstValueTestCase {
            description: "TRY_TO_DOUBLE of a long argument reads it once",
            sql: "SELECT TRY_TO_DOUBLE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(' \
                  1.5 ', 'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', '')) AS v",
            expected_outcome: Ok("1.5"),
        },
        FirstValueTestCase {
            description: "TRY_TO_DOUBLE of long unreadable text is NULL",
            sql: "SELECT \
                  TRY_TO_DOUBLE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE('x', \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', '')) IS NULL AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "TO_DOUBLE of long unreadable text fails as error 100038",
            sql: "SELECT TO_DOUBLE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE('x', \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', '')) AS v",
            expected_outcome: Err("100038"),
        },
        FirstValueTestCase {
            description: "TRY_TO_NUMBER of a long argument reads it once",
            sql: "SELECT TRY_TO_NUMBER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(' 7 \
                  ', 'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', '')) AS v",
            expected_outcome: Ok("7"),
        },
        FirstValueTestCase {
            description: "a long argument cast to BOOLEAN reads it once",
            sql: "SELECT REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE('yes', \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', '')::BOOLEAN AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "long text equal to a NUMBER reads it once",
            sql: "SELECT REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE('7.0', \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', '') = 7 AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "long text compared with a NUMBER stays unrounded",
            sql: "SELECT REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE('1.5', \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', '') = 2 AS v",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "long text beyond the NUMBER's widest scale raises",
            sql: "SELECT REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE('7.4', \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                  'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', '') = CAST(7 AS NUMBER(38,0)) AS v",
            expected_outcome: Err("text_number_scale"),
        },
        FirstValueTestCase {
            description: "SUM of a long FLOAT argument adds exact values from one list",
            sql: "SELECT SUM(IFF(x > 900007, -x, IFF(x > 900006, -x, IFF(x > 900005, -x, IFF(x > \
                  900004, -x, IFF(x > 900003, -x, IFF(x > 900002, -x, IFF(x > 900001, -x, \
                  x)))))))) AS s FROM (SELECT 0.5::FLOAT AS x UNION ALL SELECT 0.25::FLOAT UNION \
                  ALL SELECT 0.125::FLOAT UNION ALL SELECT NULL::FLOAT) AS t",
            expected_outcome: Ok("0.875"),
        },
        FirstValueTestCase {
            description: "SUM DISTINCT of a long FLOAT argument",
            sql: "SELECT SUM(DISTINCT IFF(x > 900007, -x, IFF(x > 900006, -x, IFF(x > 900005, -x, \
                  IFF(x > 900004, -x, IFF(x > 900003, -x, IFF(x > 900002, -x, IFF(x > 900001, -x, \
                  x)))))))) AS s FROM (SELECT 0.5::FLOAT AS x UNION ALL SELECT 0.25::FLOAT UNION \
                  ALL SELECT 0.125::FLOAT UNION ALL SELECT NULL::FLOAT) AS t",
            expected_outcome: Ok("0.875"),
        },
        FirstValueTestCase {
            description: "a windowed SUM of a long FLOAT argument",
            sql: "SELECT SUM(IFF(x > 900007, -x, IFF(x > 900006, -x, IFF(x > 900005, -x, IFF(x > \
                  900004, -x, IFF(x > 900003, -x, IFF(x > 900002, -x, IFF(x > 900001, -x, \
                  x)))))))) OVER () AS s FROM (SELECT 0.5::FLOAT AS x UNION ALL SELECT 0.25::FLOAT \
                  UNION ALL SELECT 0.125::FLOAT UNION ALL SELECT NULL::FLOAT) AS t",
            expected_outcome: Ok("0.875"),
        },
        FirstValueTestCase {
            description: "SUM of a long FLOAT argument whose order of addition matters raises",
            sql: "SELECT SUM(IFF(x > 900007, -x, IFF(x > 900006, -x, IFF(x > 900005, -x, IFF(x > \
                  900004, -x, IFF(x > 900003, -x, IFF(x > 900002, -x, IFF(x > 900001, -x, \
                  x)))))))) AS s FROM (SELECT 0.1::FLOAT AS x UNION ALL SELECT 0.2::FLOAT UNION \
                  ALL SELECT 0.3::FLOAT) AS t",
            expected_outcome: Err("float_sum_order"),
        },
        FirstValueTestCase {
            description: "SUM of a long FLOAT argument over only NULLs is NULL",
            sql: "SELECT SUM(IFF(x > 900007, -x, IFF(x > 900006, -x, IFF(x > 900005, -x, IFF(x > \
                  900004, -x, IFF(x > 900003, -x, IFF(x > 900002, -x, IFF(x > 900001, -x, \
                  x)))))))) IS NULL AS s FROM (SELECT NULL::FLOAT AS x) AS t",
            expected_outcome: Ok("true"),
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

#[test]
fn given_a_long_argument_when_converting_then_no_block_repeats_it() {
    let long = "REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(' 1.5 ', \
                 'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                 'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                 'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                 'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                 'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                 'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', ''), \
                 'qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq', '')";
    let test_cases = [
        ArgumentCopiesTestCase {
            description: "TRY_TO_DOUBLE writes a long argument once",
            wrapper: "SELECT TRY_TO_DOUBLE(ARGUMENT) AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "TRY_TO_NUMBER writes a long argument once",
            wrapper: "SELECT TRY_TO_NUMBER(ARGUMENT) AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "a cast to BOOLEAN writes a long argument once",
            wrapper: "SELECT ARGUMENT::BOOLEAN AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "text compared with a NUMBER writes a long argument once",
            wrapper: "SELECT ARGUMENT = 7 AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "SUM of FLOAT writes a long argument once",
            wrapper: "SELECT SUM(TRY_TO_DOUBLE(ARGUMENT)) AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "a windowed SUM of FLOAT writes a long argument once",
            wrapper: "SELECT SUM(TRY_TO_DOUBLE(ARGUMENT)) OVER () AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "SUBSTR writes a long computed start once",
            wrapper: "SELECT SUBSTR('order', LENGTH(ARGUMENT)) AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "SUBSTR writes a long text once",
            wrapper: "SELECT SUBSTR(ARGUMENT, 2) AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "LEFT writes a long computed length once",
            wrapper: "SELECT LEFT('order', LENGTH(ARGUMENT)) AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "a simple CASE writes a long operand once",
            wrapper: "SELECT CASE TRIM(ARGUMENT) WHEN 'a' THEN 1 WHEN 'b' THEN 2 WHEN 'c' THEN 3 END AS v",
            expected_most_copies: 1,
        },
        ArgumentCopiesTestCase {
            description: "an unordered ARRAY_AGG writes its value twice, once to filter",
            wrapper: "SELECT ARRAY_AGG(ARGUMENT) AS v",
            expected_most_copies: 2,
        },
    ];
    for test_case in test_cases {
        let copies = argument_copies(test_case.wrapper, "' 1.5 '", long);
        assert!(
            copies <= test_case.expected_most_copies,
            "{}: {copies} copies",
            test_case.description
        );
    }
}
