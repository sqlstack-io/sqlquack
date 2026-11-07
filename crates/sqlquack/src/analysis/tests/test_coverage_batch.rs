use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::{BuiltFirstValueTestCase, FirstValueTestCase};

#[test]
fn given_text_searches_when_converting_then_positions_count_characters_from_one() {
    let test_cases = [
        FirstValueTestCase {
            description: "CHARINDEX finds the first occurrence, counting characters",
            sql: "SELECT CHARINDEX('é', 'café crème') AS v",
            expected_outcome: Ok("4"),
        },
        FirstValueTestCase {
            description: "an absent text is 0",
            sql: "SELECT CHARINDEX('x', 'order') AS v",
            expected_outcome: Ok("0"),
        },
        FirstValueTestCase {
            description: "an empty needle is 1, also in empty text",
            sql: "SELECT CHARINDEX('', '') AS v",
            expected_outcome: Ok("1"),
        },
        FirstValueTestCase {
            description: "POSITION(s IN t) reads s in t",
            sql: "SELECT POSITION(' - ' IN 'SKU - 1042') AS v",
            expected_outcome: Ok("4"),
        },
        FirstValueTestCase {
            description: "POSITION(s, t) reads s in t",
            sql: "SELECT POSITION('-', 'SKU-1042') AS v",
            expected_outcome: Ok("4"),
        },
        FirstValueTestCase {
            description: "NULL is NULL",
            sql: "SELECT COALESCE(TO_VARCHAR(CHARINDEX(NULL, 'order')), 'null') AS v",
            expected_outcome: Ok("null"),
        },
        FirstValueTestCase {
            description: "a start position is refused",
            sql: "SELECT CHARINDEX('o', 'order', 2) AS v",
            expected_outcome: Err("start position"),
        },
        FirstValueTestCase {
            description: "a number operand is refused",
            sql: "SELECT CHARINDEX('1', 12) AS v",
            expected_outcome: Err("implicit conversion to text"),
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
fn given_computed_subscripts_when_converting_then_they_read_as_get() {
    let test_cases = [
        FirstValueTestCase {
            description: "an index column reads the array element, as GET does",
            sql: "SELECT PARSE_JSON('[\"a\",\"b\"]')[customer_id]::VARCHAR AS v FROM customers",
            expected_outcome: Ok("b"),
        },
        FirstValueTestCase {
            description: "a computed index past the end is NULL",
            sql: "SELECT COALESCE(PARSE_JSON('[\"a\"]')[customer_id + 5]::VARCHAR, 'null') AS v \
                  FROM customers",
            expected_outcome: Ok("null"),
        },
        FirstValueTestCase {
            description: "a key column reads the object's value under that key",
            sql: "SELECT PARSE_JSON('{\"1\":\"x\"}')[note]::VARCHAR AS v FROM customers",
            expected_outcome: Ok("x"),
        },
        FirstValueTestCase {
            description: "a negative constant index stays refused",
            sql: "SELECT PARSE_JSON('[1]')[-1] AS v",
            expected_outcome: Err("other than a literal"),
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
fn given_mode_when_converting_then_one_most_frequent_value_converts_and_ties_raise() {
    let test_cases = [
        FirstValueTestCase {
            description: "the most frequent value, NULLs ignored",
            sql: "SELECT MODE(amount) AS v FROM (SELECT 1 AS g, amount FROM orders UNION ALL \
                  SELECT 1, 7.50 UNION ALL SELECT 1, NULL UNION ALL SELECT 1, NULL UNION ALL \
                  SELECT 1, NULL) AS t GROUP BY g",
            expected_outcome: Ok("7.50"),
        },
        FirstValueTestCase {
            description: "values tied for most frequent raise mode_ties",
            sql: "SELECT MODE(amount) AS v FROM orders GROUP BY amount > 0",
            expected_outcome: Err("mode_ties"),
        },
        FirstValueTestCase {
            description: "a FLOAT group holding -0 and 0 raises mode_ties",
            sql: "SELECT MODE(x) AS v FROM (SELECT 1 AS g, -0.0::FLOAT AS x UNION ALL \
                  SELECT 1, 0.0::FLOAT UNION ALL SELECT 1, 1.0::FLOAT) AS t GROUP BY g",
            expected_outcome: Err("mode_ties"),
        },
        FirstValueTestCase {
            description: "MODE without GROUP BY over no rows raises: Snowflake answers by plan",
            sql: "SELECT MODE(amount) AS v FROM orders WHERE amount > 100",
            expected_outcome: Err("mode_of_no_rows"),
        },
        FirstValueTestCase {
            description: "MODE without GROUP BY of text over no rows raises",
            sql: "SELECT MODE(note) = '' AS v FROM customers WHERE customer_id > 100",
            expected_outcome: Err("mode_of_no_rows"),
        },
        FirstValueTestCase {
            description: "MODE without GROUP BY over values is the most frequent one",
            sql: "SELECT MODE(customer_id) AS v FROM orders",
            expected_outcome: Ok("2"),
        },
        FirstValueTestCase {
            description: "MODE without GROUP BY of a timestamp over no rows raises",
            sql: "SELECT MODE(TO_TIMESTAMP_NTZ(note)) AS v FROM customers WHERE customer_id > 100",
            expected_outcome: Err("mode_of_no_rows"),
        },
        FirstValueTestCase {
            description: "MODE over a window stays refused",
            sql: "SELECT MODE(amount) OVER (PARTITION BY customer_id) AS v FROM orders",
            expected_outcome: Err("MODE"),
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
fn given_extracting_regexps_when_converting_then_groups_lines_and_control_escapes_read_as_recorded()
{
    let test_cases = [
        FirstValueTestCase {
            description: "a group without e extracts the group",
            sql: "SELECT REGEXP_SUBSTR_ALL('/1/22', '/([0-9]+)', 1, 1, 'c', 1)::VARCHAR AS v",
            expected_outcome: Ok("[\"1\",\"22\"]"),
        },
        FirstValueTestCase {
            description: "m lets ^ and $ match at line feeds",
            sql: "SELECT REGEXP_SUBSTR_ALL('gift' || CHR(10) || 'card', '^[a-z]+$', 1, 1, 'm')::VARCHAR \
                  AS v",
            expected_outcome: Ok("[\"gift\",\"card\"]"),
        },
        FirstValueTestCase {
            description: "\\t in a pattern matches a tab",
            sql: "SELECT REGEXP_SUBSTR('sku' || CHR(9) || '42', 'sku\\\\t([0-9]+)', 1, 1, 'e') AS v",
            expected_outcome: Ok("42"),
        },
        FirstValueTestCase {
            description: "an unrecorded parameter stays refused",
            sql: "SELECT REGEXP_SUBSTR('a', 'a', 1, 1, 'x') AS v",
            expected_outcome: Err("parameters other than"),
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
fn given_formatted_and_try_cast_timestamps_when_converting_then_text_reads_as_recorded() {
    let test_cases = [
        FirstValueTestCase {
            description: "TO_TIMESTAMP with a format reads text spelled in it",
            sql: "SELECT TO_TIMESTAMP('2024-02-29 9:5:1', 'YYYY-MM-DD HH24:MI:SS')::VARCHAR AS v",
            expected_outcome: Ok("2024-02-29 09:05:01.000"),
        },
        FirstValueTestCase {
            description: "TO_TIMESTAMP with a format raises Snowflake's error for empty text",
            sql: "SELECT TO_TIMESTAMP('', 'YYYY-MM-DD HH24:MI:SS') AS v",
            expected_outcome: Err("100096"),
        },
        FirstValueTestCase {
            description: "TO_TIMESTAMP with a format beyond DuckDB's nanosecond range raises timestamp_range",
            sql: "SELECT TO_TIMESTAMP('0001-01-01 00:00:00', 'YYYY-MM-DD HH24:MI:SS') AS v",
            expected_outcome: Err("timestamp_range"),
        },
        FirstValueTestCase {
            description: "TRY_TO_TIMESTAMP with a format beyond DuckDB's nanosecond range raises too",
            sql: "SELECT TRY_TO_TIMESTAMP('9999-12-31 23:59:59', 'YYYY-MM-DD HH24:MI:SS') AS v",
            expected_outcome: Err("timestamp_range"),
        },
        FirstValueTestCase {
            description: "TO_TIMESTAMP_NTZ with a format raises Snowflake's error for an impossible date",
            sql: "SELECT TO_TIMESTAMP_NTZ('2023-02-29 10:00:00', 'YYYY-MM-DD HH24:MI:SS') AS v",
            expected_outcome: Err("100096"),
        },
        FirstValueTestCase {
            description: "TRY_TO_TIMESTAMP with a format is NULL for an impossible date",
            sql: "SELECT COALESCE(TRY_TO_TIMESTAMP('2023-02-29 10:00:00', 'YYYY-MM-DD HH24:MI:SS')\
                  ::VARCHAR, 'null') AS v",
            expected_outcome: Ok("null"),
        },
        FirstValueTestCase {
            description: "TO_TIMESTAMP with a scale still reads an epoch count",
            sql: "SELECT TO_TIMESTAMP(customer_id * 1000, 3) = '1970-01-01 00:00:01'::TIMESTAMP_NTZ(3) \
                  AS v FROM customers",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "TRY_CAST of text to TIMESTAMP reads it as TRY_TO_TIMESTAMP_NTZ does",
            sql: "SELECT TRY_CAST(note || '000000000' AS TIMESTAMP)::VARCHAR AS v FROM customers",
            expected_outcome: Ok("2001-09-09 01:46:40.000"),
        },
        FirstValueTestCase {
            description: "TRY_CAST of digitless text to TIMESTAMP is NULL",
            sql: "SELECT COALESCE(TRY_CAST(SUBSTR(note, 2) || 'n/a' AS TIMESTAMP)::VARCHAR, 'null') AS v \
                  FROM customers",
            expected_outcome: Ok("null"),
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
fn given_times_of_timestamps_when_converting_then_wall_clock_times_compare_and_subtract() {
    let test_cases = [
        FirstValueTestCase {
            description: "TO_TIME of a timestamp is its time of day",
            sql: "SELECT TO_TIME('2024-02-29 13:45:01.5'::TIMESTAMP_NTZ)::VARCHAR AS v",
            expected_outcome: Ok("13:45:01"),
        },
        FirstValueTestCase {
            description: "TIME of a timestamp before the epoch keeps its wall-clock time",
            sql: "SELECT TIME('1969-12-31 23:59:59'::TIMESTAMP_NTZ)::VARCHAR AS v",
            expected_outcome: Ok("23:59:59"),
        },
        FirstValueTestCase {
            description: "a TIME compared with HH:MI:SS text reads the text as a TIME",
            sql: "SELECT TIME('2024-02-29 00:00:00'::TIMESTAMP_NTZ) <> '00:00:00' AS v",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "a TIME compared with other text stays refused",
            sql: "SELECT TIME('2024-02-29 00:00:00'::TIMESTAMP_NTZ) <> '0:00' AS v",
            expected_outcome: Err("implicit conversion"),
        },
        FirstValueTestCase {
            description: "DATEDIFF in microseconds between TIMEs",
            sql: "SELECT DATEDIFF('microsecond', '00:00:00'::TIME, '00:01:35.42'::TIME) AS v",
            expected_outcome: Ok("95420000"),
        },
        FirstValueTestCase {
            description: "DATEDIFF in hours between TIMEs counts hour boundaries",
            sql: "SELECT DATEDIFF(hour, '00:59:59'::TIME, '01:00:00'::TIME) AS v",
            expected_outcome: Ok("1"),
        },
        FirstValueTestCase {
            description: "TO_TIME of a zoned timestamp stays refused",
            sql: "SELECT TO_TIME(CONVERT_TIMEZONE('Europe/Paris', '2024-01-01 10:00:00'::TIMESTAMP_NTZ)) \
                  AS v",
            expected_outcome: Err("TO_TIME of"),
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
fn given_interval_literals_when_converting_then_dates_and_timestamps_move_by_their_units() {
    let test_cases = [
        FirstValueTestCase {
            description: "a DATE minus days stays a DATE",
            sql: "SELECT ('2024-03-05'::DATE - INTERVAL '7 days')::VARCHAR AS v",
            expected_outcome: Ok("2024-02-27"),
        },
        FirstValueTestCase {
            description: "a DATE plus days written after the quotes is a TIMESTAMP_NTZ at midnight",
            sql: "SELECT ('2026-04-01'::DATE + INTERVAL '3' DAYS)::VARCHAR AS v",
            expected_outcome: Ok("2026-04-04 00:00:00.000"),
        },
        FirstValueTestCase {
            description: "a DATE minus one day written after the quotes is a TIMESTAMP_NTZ too",
            sql: "SELECT ('2024-03-01'::DATE - INTERVAL '1' DAY)::VARCHAR AS v",
            expected_outcome: Ok("2024-02-29 00:00:00.000"),
        },
        FirstValueTestCase {
            description: "a timestamp plus hours crosses midnight",
            sql: "SELECT ('2024-02-28 23:30:00'::TIMESTAMP_NTZ + INTERVAL '2 hours')::VARCHAR AS v",
            expected_outcome: Ok("2024-02-29 01:30:00.000"),
        },
        FirstValueTestCase {
            description: "an interval of an unquoted number is refused",
            sql: "SELECT '2024-03-05'::DATE + INTERVAL 3 DAY AS v",
            expected_outcome: Err("not a quoted string"),
        },
        FirstValueTestCase {
            description: "a DATE plus hours stays refused",
            sql: "SELECT '2024-03-05'::DATE + INTERVAL '2 hours' AS v",
            expected_outcome: Err("an INTERVAL in HOUR added to DATE"),
        },
        FirstValueTestCase {
            description: "an interval in months stays refused",
            sql: "SELECT '2024-03-05'::DATE - INTERVAL '1 month' AS v",
            expected_outcome: Err("an INTERVAL other than"),
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
fn given_group_by_all_when_converting_then_items_without_aggregates_are_the_keys() {
    let test_cases = [
        FirstValueTestCase {
            description: "GROUP BY ALL groups by the items that hold no aggregate",
            sql: "SELECT COUNT(*) AS v FROM (SELECT customer_id, SUM(amount) AS total, \
                  COUNT_IF(amount > 3) AS large FROM orders GROUP BY ALL) AS t",
            expected_outcome: Ok("2"),
        },
        FirstValueTestCase {
            description: "GROUP BY ALL beside a star stays refused",
            sql: "SELECT * FROM orders GROUP BY ALL",
            expected_outcome: Err("beside a star"),
        },
        FirstValueTestCase {
            description: "GROUP BY ALL of only aggregates stays refused",
            sql: "SELECT COUNT(*) AS v FROM orders GROUP BY ALL",
            expected_outcome: Err("every SELECT item is an aggregate"),
        },
        FirstValueTestCase {
            description: "GROUP BY ALL of an item holding a window function stays refused",
            sql: "SELECT customer_id, ROW_NUMBER() OVER (ORDER BY customer_id) AS r FROM orders \
                  GROUP BY ALL",
            expected_outcome: Err("window function or subquery"),
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
fn given_simple_case_of_a_computed_operand_when_converting_then_the_first_equal_when_wins() {
    let test_cases = [
        FirstValueTestCase {
            description: "a computed operand equal to a later WHEN takes that WHEN's result",
            sql: "SELECT CASE UPPER(note) WHEN 'A' THEN 'first' WHEN '7' THEN 'second' \
                  ELSE 'other' END AS v FROM customers WHERE customer_id = 2",
            expected_outcome: Ok("second"),
        },
        FirstValueTestCase {
            description: "a computed operand equal to no WHEN takes the ELSE",
            sql: "SELECT CASE UPPER(note) WHEN 'A' THEN 'first' ELSE 'other' END AS v \
                  FROM customers",
            expected_outcome: Ok("other"),
        },
        FirstValueTestCase {
            description: "a computed operand equal to two WHENs takes the first",
            sql: "SELECT CASE customer_id * 2 WHEN 2 THEN 'first' WHEN 2.0 THEN 'second' END \
                  AS v FROM customers WHERE customer_id = 1",
            expected_outcome: Ok("first"),
        },
        FirstValueTestCase {
            description: "a computed operand compared with a column WHEN reads that row's column",
            sql: "SELECT CASE customer_id * 1 WHEN customer_id THEN 'same' ELSE 'other' END AS v \
                  FROM customers",
            expected_outcome: Ok("same"),
        },
        FirstValueTestCase {
            description: "a NULL computed operand matches no WHEN, not even NULL",
            sql: "SELECT CASE UPPER(NULL) WHEN NULL THEN 'null' ELSE 'other' END AS v",
            expected_outcome: Ok("other"),
        },
        FirstValueTestCase {
            description: "an aggregate operand converts",
            sql: "SELECT CASE COUNT(*) WHEN 3 THEN 'three' END AS v FROM orders",
            expected_outcome: Ok("three"),
        },
        FirstValueTestCase {
            description: "a windowed operand converts",
            sql: "SELECT CASE ROW_NUMBER() OVER (ORDER BY amount) WHEN 1 THEN amount END AS v \
                  FROM orders ORDER BY v NULLS LAST",
            expected_outcome: Ok("2.50"),
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
fn given_listagg_when_rows_tie_on_its_keys_then_only_equal_tied_values_convert() {
    let test_cases = [
        FirstValueTestCase {
            description: "distinct keys order the values",
            sql: "SELECT LISTAGG(TO_VARCHAR(customer_id), ',') WITHIN GROUP (ORDER BY amount) AS v \
                  FROM orders",
            expected_outcome: Ok("2,1,2"),
        },
        FirstValueTestCase {
            description: "tied keys holding equal values convert",
            sql: "SELECT LISTAGG(TO_VARCHAR(customer_id), ',') \
                  WITHIN GROUP (ORDER BY customer_id) AS v FROM orders",
            expected_outcome: Ok("1,2,2"),
        },
        FirstValueTestCase {
            description: "tied keys holding different values raise",
            sql: "SELECT LISTAGG(TO_VARCHAR(amount), ',') WITHIN GROUP (ORDER BY customer_id) AS v \
                  FROM orders",
            expected_outcome: Err("LISTAGG of different values tied on its order"),
        },
        FirstValueTestCase {
            description: "tied keys holding different values raise over a window",
            sql: "SELECT LISTAGG(TO_VARCHAR(amount), ',') WITHIN GROUP (ORDER BY customer_id) \
                  OVER (PARTITION BY customer_id) AS v FROM orders ORDER BY customer_id DESC",
            expected_outcome: Err("LISTAGG of different values tied on its order"),
        },
        FirstValueTestCase {
            description: "a second key breaking the tie converts",
            sql: "SELECT LISTAGG(TO_VARCHAR(customer_id), ',') \
                  WITHIN GROUP (ORDER BY customer_id, amount) AS v FROM orders",
            expected_outcome: Ok("1,2,2"),
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
fn given_ordered_array_agg_when_rows_tie_on_its_keys_then_only_equal_tied_values_convert() {
    let test_cases = [
        FirstValueTestCase {
            description: "distinct keys order the values",
            sql: "SELECT ARRAY_AGG(amount) WITHIN GROUP (ORDER BY amount DESC) AS v FROM orders",
            expected_outcome: Ok("[7.5,5,2.5]"),
        },
        FirstValueTestCase {
            description: "tied keys holding equal values convert",
            sql: "SELECT ARRAY_AGG(customer_id) WITHIN GROUP (ORDER BY customer_id) AS v \
                  FROM orders",
            expected_outcome: Ok("[1,2,2]"),
        },
        FirstValueTestCase {
            description: "tied keys holding different values raise",
            sql: "SELECT ARRAY_AGG(amount) WITHIN GROUP (ORDER BY customer_id) AS v FROM orders",
            expected_outcome: Err("ARRAY_AGG WITHIN GROUP of different values tied on its order"),
        },
        FirstValueTestCase {
            description: "a second key breaking the tie converts",
            sql: "SELECT ARRAY_AGG(amount) WITHIN GROUP (ORDER BY customer_id, amount) AS v \
                  FROM orders",
            expected_outcome: Ok("[5,2.5,7.5]"),
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
fn given_arrays_built_with_sql_nulls_when_sizing_them_then_undefined_elements_count() {
    let test_cases = [
        FirstValueTestCase {
            description: "ARRAY_APPEND of a SQL NULL adds one element",
            sql: "SELECT ARRAY_SIZE(ARRAY_APPEND(SPLIT('a,b', ','), NULL::NUMBER)) AS v",
            expected_outcome: Ok("3"),
        },
        FirstValueTestCase {
            description: "ARRAY_CAT of constructed arrays with SQL NULLs counts each",
            sql: "SELECT ARRAY_SIZE(ARRAY_CAT(ARRAY_CONSTRUCT(NULL::VARCHAR), \
                  ARRAY_CONSTRUCT(NULL::VARCHAR, 'b'))) AS v",
            expected_outcome: Ok("3"),
        },
        FirstValueTestCase {
            description: "an array read by ARRAY_DISTINCT before ARRAY_SIZE keeps its guard",
            sql: "SELECT ARRAY_SIZE(ARRAY_DISTINCT(ARRAY_CONSTRUCT(NULL::VARCHAR, 'b'))) AS v",
            expected_outcome: Err("keeps as undefined"),
        },
        FirstValueTestCase {
            description: "a FLOAT padded to ten digits at scale 6 rounds exactly",
            sql: "SELECT (1649.73::FLOAT)::NUMBER(18,6) AS v",
            expected_outcome: Ok("1649.730000"),
        },
        FirstValueTestCase {
            description: "a FLOAT reaching eleven digits once scaled still raises",
            sql: "SELECT (1649.73::FLOAT)::NUMBER(38,8) AS v",
            expected_outcome: Err("float_decimal_digits"),
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
fn given_dates_and_naive_timestamps_when_converting_to_timestamp_then_values_are_kept() {
    let test_cases = [
        FirstValueTestCase {
            description: "TO_TIMESTAMP_NTZ of a TIMESTAMP_NTZ(9) is the same timestamp",
            sql: "SELECT TO_TIMESTAMP_NTZ('2024-02-29 13:45:01.123456789'::TIMESTAMP_NTZ(9)) \
                  = '2024-02-29 13:45:01.123456789'::TIMESTAMP_NTZ(9) AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "TO_TIMESTAMP_NTZ of a TIMESTAMP_NTZ(3) keeps its milliseconds",
            sql: "SELECT TO_TIMESTAMP_NTZ('2024-02-29 13:45:01.123'::TIMESTAMP_NTZ(3)) \
                  = '2024-02-29 13:45:01.123'::TIMESTAMP_NTZ(9) AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "TO_TIMESTAMP of a DATE is its midnight",
            sql: "SELECT TO_TIMESTAMP('2024-02-29'::DATE) \
                  = '2024-02-29 00:00:00'::TIMESTAMP_NTZ(9) AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "TRY_TO_TIMESTAMP_NTZ of a timestamp is refused, as Snowflake rejects it",
            sql: "SELECT TRY_TO_TIMESTAMP_NTZ('2024-02-29 13:45:01'::TIMESTAMP_NTZ(9)) AS v",
            expected_outcome: Err("TRY_TO_TIMESTAMP_NTZ of TIMESTAMP_NTZ(9)"),
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
fn given_quoted_units_when_converting_timestampdiff_then_boundaries_are_counted() {
    let test_cases = [
        FirstValueTestCase {
            description: "a quoted minute counts the boundary crossed, not whole minutes elapsed",
            sql: "SELECT TIMESTAMPDIFF('minute', '2024-01-31 23:59:59'::TIMESTAMP_NTZ, \
                  '2024-02-01 00:00:01'::TIMESTAMP_NTZ) AS v",
            expected_outcome: Ok("1"),
        },
        FirstValueTestCase {
            description: "a quoted unit in mixed case reads as the unit",
            sql: "SELECT TIMESTAMPDIFF('Day', '2024-02-29 23:00:00'::TIMESTAMP_NTZ, \
                  '2024-02-28 01:00:00'::TIMESTAMP_NTZ) AS v",
            expected_outcome: Ok("-1"),
        },
        FirstValueTestCase {
            description: "a quoted unknown unit refuses",
            sql: "SELECT TIMESTAMPDIFF('fortnight', '2024-01-01'::TIMESTAMP_NTZ, \
                  '2024-02-01'::TIMESTAMP_NTZ) AS v",
            expected_outcome: Err("FORTNIGHT"),
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
fn given_dates_when_converting_dayofweekiso_then_monday_is_one_and_sunday_seven() {
    let test_cases = [
        FirstValueTestCase {
            description: "a Sunday is 7",
            sql: "SELECT DAYOFWEEKISO('2023-12-31'::DATE) AS v",
            expected_outcome: Ok("7"),
        },
        FirstValueTestCase {
            description: "a Monday is 1",
            sql: "SELECT DAYOFWEEKISO('2024-01-01'::DATE) AS v",
            expected_outcome: Ok("1"),
        },
        FirstValueTestCase {
            description: "a timestamp just before midnight keeps its own day",
            sql: "SELECT DAYOFWEEKISO('2024-02-29 23:59:59.999999999'::TIMESTAMP_NTZ(9)) AS v",
            expected_outcome: Ok("4"),
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
fn given_ordered_rows_when_converting_ntile_then_extra_rows_go_to_the_first_buckets() {
    let test_cases = [
        FirstValueTestCase {
            description: "the third of seven rows in three buckets is in the first",
            sql: "SELECT b FROM (SELECT x, NTILE(3) OVER (ORDER BY x) AS b \
                  FROM (SELECT SEQ4() AS x FROM TABLE(GENERATOR(ROWCOUNT => 7)))) WHERE x = 2",
            expected_outcome: Ok("1"),
        },
        FirstValueTestCase {
            description: "a bucket count that is not a literal refuses",
            sql: "SELECT NTILE(x) OVER (ORDER BY x) AS b \
                  FROM (SELECT SEQ4() AS x FROM TABLE(GENERATOR(ROWCOUNT => 3)))",
            expected_outcome: Err("positive integer literal"),
        },
        FirstValueTestCase {
            description: "NTILE without an ORDER BY refuses",
            sql: "SELECT NTILE(2) OVER () AS b \
                  FROM (SELECT SEQ4() AS x FROM TABLE(GENERATOR(ROWCOUNT => 3)))",
            expected_outcome: Err("ORDER BY"),
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
fn given_dates_when_converting_last_day_then_the_month_end_is_kept() {
    let test_cases = [
        FirstValueTestCase {
            description: "a leap February ends on the 29th",
            sql: "SELECT LAST_DAY('2024-02-10'::DATE) = '2024-02-29'::DATE AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a timestamp just before midnight keeps its own month",
            sql: "SELECT LAST_DAY('2024-01-31 23:59:59.999999999'::TIMESTAMP_NTZ(9)) \
                  = '2024-01-31'::DATE AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a quoted month unit is the default",
            sql: "SELECT LAST_DAY('2023-02-10'::DATE, 'month') = '2023-02-28'::DATE AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a week unit refuses",
            sql: "SELECT LAST_DAY('2024-02-10'::DATE, week) AS v",
            expected_outcome: Err("other than MONTH"),
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
fn given_sides_qualified_by_the_walked_relation_when_converting_connect_by_then_it_walks() {
    let test_cases = [
        FirstValueTestCase {
            description: "PRIOR on the left, both sides qualified",
            sql: "WITH t AS (SELECT * FROM (VALUES (1, NULL), (2, 1), (3, 2), (4, 1)) AS v(id, parent)) \
                  SELECT SUM(id * 10 + depth) AS v FROM (SELECT c.id, LEVEL AS depth FROM t AS c \
                  START WITH c.parent IS NULL CONNECT BY PRIOR c.id = c.parent)",
            expected_outcome: Ok("108"),
        },
        FirstValueTestCase {
            description: "PRIOR on the right, the alias in another case",
            sql: "WITH t AS (SELECT * FROM (VALUES (1, NULL), (2, 1), (3, 2), (4, 1)) AS v(id, parent)) \
                  SELECT SUM(id * 10 + depth) AS v FROM (SELECT c.id, LEVEL AS depth FROM t AS c \
                  START WITH c.parent IS NULL CONNECT BY C.parent = PRIOR C.id)",
            expected_outcome: Ok("108"),
        },
        FirstValueTestCase {
            description: "a qualifier naming another relation refuses",
            sql: "WITH t AS (SELECT * FROM (VALUES (1, NULL), (2, 1), (3, 2), (4, 1)) AS v(id, parent)) \
                  SELECT SUM(id * 10 + depth) AS v FROM (SELECT c.id, LEVEL AS depth FROM t AS c \
                  START WITH c.parent IS NULL CONNECT BY PRIOR x.id = c.parent)",
            expected_outcome: Err("walked relation"),
        },
        FirstValueTestCase {
            description: "ORDER BY a qualified column after the walk refuses, as Snowflake rejects it",
            sql: "WITH t AS (SELECT * FROM (VALUES (1, NULL), (2, 1)) AS v(id, parent)) \
                  SELECT c.id AS v FROM t AS c START WITH c.parent IS NULL \
                  CONNECT BY PRIOR c.id = c.parent ORDER BY c.id",
            expected_outcome: Err("Snowflake rejects"),
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
fn given_powers_of_ten_when_converting_power_then_the_nearest_double_is_kept() {
    let test_cases = [
        FirstValueTestCase {
            description: "10^23, an exact tie, rounds to even as Snowflake records",
            sql: "SELECT POWER(10, 23) = '1e23'::FLOAT AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a negative whole exponent",
            sql: "SELECT POWER(10, -3) = '0.001'::FLOAT AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "an exponent far beyond the double range is inf",
            sql: "SELECT POWER(10, 1000) = 'inf'::FLOAT AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "another base refuses",
            sql: "SELECT POWER(3, 3) AS v",
            expected_outcome: Err("other than the constant 10"),
        },
        FirstValueTestCase {
            description: "a fractional exponent refuses",
            sql: "SELECT POWER(10, 1.5) AS v",
            expected_outcome: Err("exponent"),
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
fn given_zoned_timestamps_when_counting_seconds_and_minutes_then_utc_instants_are_compared() {
    let test_cases = [
        FirstValueTestCase {
            description: "seconds between +02:00 and +00:00 values are those between the instants",
            sql: "SELECT DATEDIFF('second', '2024-06-01 12:00:00 +02:00'::TIMESTAMP_TZ, \
                  '2024-06-01 10:00:30 +00:00'::TIMESTAMP_TZ) AS v",
            expected_outcome: Ok("30"),
        },
        FirstValueTestCase {
            description: "a minute boundary crossed at a +05:30 offset, as recorded",
            sql: "SELECT DATEDIFF('minute', '2024-06-01 15:29:59 +05:30'::TIMESTAMP_TZ, \
                  '2024-06-01 10:00:01 +00:00'::TIMESTAMP_TZ) AS v",
            expected_outcome: Ok("1"),
        },
        FirstValueTestCase {
            description: "hours between zoned values refuse",
            sql: "SELECT DATEDIFF('hour', '2024-06-01 10:59:00 +05:30'::TIMESTAMP_TZ, \
                  '2024-06-01 11:01:00 +05:30'::TIMESTAMP_TZ) AS v",
            expected_outcome: Err("which reads the local time of an offset"),
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
fn given_batch_two_constructs_when_converting_then_they_match_their_recordings() {
    let test_cases = [
        FirstValueTestCase {
            description: "RATIO_TO_REPORT rounds 4/7 to six places, not truncating it",
            sql: "SELECT r AS v FROM (SELECT x, RATIO_TO_REPORT(x) OVER () AS r \
                  FROM (VALUES (4), (3)) AS t(x)) WHERE x = 4",
            expected_outcome: Ok("0.571429"),
        },
        FirstValueTestCase {
            description: "RATIO_TO_REPORT of a zero-sum partition is NULL, not a division error",
            sql: "SELECT COALESCE(RATIO_TO_REPORT(x) OVER (), -1) AS v FROM (VALUES (0), (0)) AS t(x)",
            expected_outcome: Ok("-1.000000"),
        },
        FirstValueTestCase {
            description: "RATIO_TO_REPORT over an ordered window refuses",
            sql: "SELECT RATIO_TO_REPORT(x) OVER (ORDER BY x) AS v FROM (VALUES (1), (2)) AS t(x)",
            expected_outcome: Err("Snowflake rejects"),
        },
        FirstValueTestCase {
            description: "IS_DECIMAL of a JSON fraction is true",
            sql: "SELECT IS_DECIMAL(PARSE_JSON('12.5')) AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "IS_INTEGER of a JSON fraction is false",
            sql: "SELECT IS_INTEGER(PARSE_JSON('12.5')) AS v",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "IS_INTEGER of 2.0 is true, as PARSE_JSON reads it as an integer",
            sql: "SELECT IS_INTEGER(PARSE_JSON('2.0')) AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "IS_DECIMAL of numeric text in JSON is false",
            sql: "SELECT IS_DECIMAL(PARSE_JSON('\"12\"')) AS v",
            expected_outcome: Ok("false"),
        },
        FirstValueTestCase {
            description: "\\b treats an underscore as a word character",
            sql: "SELECT COALESCE(REGEXP_SUBSTR('_AB123_', '\\\\b[A-Z]{2}[0-9]{3}\\\\b'), 'none') AS v",
            expected_outcome: Ok("none"),
        },
        FirstValueTestCase {
            description: "\\b matches beside a non-ASCII letter",
            sql: "SELECT REGEXP_SUBSTR('éAB123é', '\\\\b[A-Z]{2}[0-9]{3}\\\\b') AS v",
            expected_outcome: Ok("AB123"),
        },
        FirstValueTestCase {
            description: "a RANGE frame of 2 days spans the leap day and a later peer, unlike ROWS",
            sql: "SELECT s AS v FROM (SELECT d, x, SUM(x) OVER (ORDER BY d RANGE BETWEEN \
                  INTERVAL '2 days' PRECEDING AND CURRENT ROW) AS s FROM (VALUES \
                  ('2024-02-26'::DATE, 1), ('2024-02-28'::DATE, 2), ('2024-03-01'::DATE, 4), \
                  ('2024-03-01'::DATE, 8)) AS t(d, x)) WHERE x = 4",
            expected_outcome: Ok("14"),
        },
        FirstValueTestCase {
            description: "a RANGE frame over a descending key refuses",
            sql: "SELECT SUM(x) OVER (ORDER BY d DESC RANGE BETWEEN INTERVAL '2 days' \
                  PRECEDING AND CURRENT ROW) AS v FROM (VALUES ('2024-03-01'::DATE, 1)) AS t(d, x)",
            expected_outcome: Err("one ascending DATE key"),
        },
        FirstValueTestCase {
            description: "YY 69 is 2069 under DD-MON-YY",
            sql: "SELECT YEAR(TRY_TO_TIMESTAMP_NTZ('05-mar-69 00:00', 'DD-MON-YY HH24:MI')) AS v",
            expected_outcome: Ok("2069"),
        },
        FirstValueTestCase {
            description: "YY 70 is 1970 and a full month name reads under MON",
            sql: "SELECT YEAR(TRY_TO_TIMESTAMP_NTZ('31-December-70 23:59', \
                  'DD-MON-YY HH24:MI')) AS v",
            expected_outcome: Ok("1970"),
        },
        FirstValueTestCase {
            description: "a four-digit year where YY reads two is NULL, as recorded, not a raise",
            sql: "SELECT TRY_TO_TIMESTAMP_NTZ('05-Mar-2024 14:30', 'DD-MON-YY HH24:MI') IS NULL AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "MON outside the TRY_ forms refuses",
            sql: "SELECT TO_TIMESTAMP_NTZ('05-Mar-24 10:00', 'DD-MON-YY HH24:MI') AS v",
            expected_outcome: Err("recorded only under the TRY_ forms"),
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
fn given_direct_array_constructs_when_writing_them_as_text_then_null_elements_are_undefined() {
    let test_cases = [
        FirstValueTestCase {
            description: "a NULL first element is written undefined, a string quoted",
            sql: "SELECT ARRAY_CONSTRUCT(NULL, 'G-2')::VARCHAR AS v",
            expected_outcome: Ok("[undefined,\"G-2\"]"),
        },
        FirstValueTestCase {
            description: "ARRAY_TO_STRING writes an undefined element as empty text",
            sql: "SELECT ARRAY_TO_STRING(ARRAY_CONSTRUCT('gift', NULL), '|') AS v",
            expected_outcome: Ok("gift|"),
        },
        FirstValueTestCase {
            description: "a JSON null element is null, not undefined",
            sql: "SELECT ARRAY_CONSTRUCT(PARSE_JSON('null'), NULL)::VARCHAR AS v",
            expected_outcome: Ok("[null,undefined]"),
        },
        FirstValueTestCase {
            description: "an ARRAY_CONSTRUCT not written directly in the cast still raises",
            sql: "SELECT a::VARCHAR AS v FROM (SELECT ARRAY_CONSTRUCT(NULL, 'x') AS a)",
            expected_outcome: Err("keeps as undefined"),
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
fn given_pivot_in_from_when_converting_then_each_listed_key_aggregates_its_group_rows() {
    let source = "(SELECT customer_id, IFF(amount > 3, 'big', 'small') AS k, amount FROM orders)";
    let pivoted = |aggregate: &str, keys: &str, alias: &str| {
        format!("{source} PIVOT({aggregate}(amount) FOR k IN ({keys})) AS p{alias}")
    };
    let test_cases = [
        BuiltFirstValueTestCase {
            description: "SUM of a key sums only that key's rows in the group",
            sql: format!(
                "SELECT p.big || '/' || p.small AS v FROM {} WHERE p.customer_id = 2",
                pivoted("SUM", "'big', 'small'", "(customer_id, big, small)")
            ),
            expected_outcome: Ok("7.50/2.50"),
        },
        BuiltFirstValueTestCase {
            description: "a listed key without rows in the group is NULL",
            sql: format!(
                "SELECT COALESCE(TO_VARCHAR(p.small), 'null') AS v FROM {} WHERE p.customer_id = 1",
                pivoted("MAX", "'big', 'small'", "(customer_id, big, small)")
            ),
            expected_outcome: Ok("null"),
        },
        BuiltFirstValueTestCase {
            description: "a key matches exactly, so a differently cased key is NULL",
            sql: format!(
                "SELECT COALESCE(TO_VARCHAR(p.big), 'null') AS v FROM {} WHERE p.customer_id = 1",
                pivoted("SUM", "'BIG'", "(customer_id, big)")
            ),
            expected_outcome: Ok("null"),
        },
        BuiltFirstValueTestCase {
            description: "without an alias column list it stays refused",
            sql: format!("SELECT * FROM {}", pivoted("SUM", "'big'", "")),
            expected_outcome: Err("alias column list"),
        },
        BuiltFirstValueTestCase {
            description: "another aggregate stays refused",
            sql: format!(
                "SELECT * FROM {}",
                pivoted("AVG", "'big'", "(customer_id, big)")
            ),
            expected_outcome: Err("other than SUM or MAX"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_first_outcome(&test_case.sql, test_case.expected_outcome),
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
fn given_keys_beyond_plain_names_when_converting_flatten_then_paths_are_written_as_recorded() {
    let path = |json: &str, recursive: bool, filter: &str| {
        format!(
            "SELECT f.path AS v FROM (SELECT 1 AS x) AS t, LATERAL FLATTEN(input => PARSE_JSON('{json}'), \
             recursive => {recursive}) AS f WHERE {filter}"
        )
    };
    let test_cases = [
        BuiltFirstValueTestCase {
            description: "an accented name is written as it is",
            sql: path(r#"{"éa": 1}"#, false, "TRUE"),
            expected_outcome: Ok("éa"),
        },
        BuiltFirstValueTestCase {
            description: "a key with a quote is bracketed, the quote escaped",
            sql: path(r#"{"it''s": 1}"#, false, "TRUE"),
            expected_outcome: Ok(r"['it\'s']"),
        },
        BuiltFirstValueTestCase {
            description: "a name below a bracketed key keeps its dot, the leading dot dropped",
            sql: path(r#"{"a_1": {"b c": {"d": 6}}}"#, true, "f.key = 'd'"),
            expected_outcome: Ok("a_1['b c'].d"),
        },
        BuiltFirstValueTestCase {
            description: "an index below a bracketed key holding [ follows the bracket",
            sql: path(r#"{"a[b": [3]}"#, true, "f.index = 0"),
            expected_outcome: Ok("['a[b'][0]"),
        },
        BuiltFirstValueTestCase {
            description: "a key with a double quote is bracketed as it is",
            sql: path(r#"{"q\\"t": 3}"#, true, "TRUE"),
            expected_outcome: Ok(r#"['q"t']"#),
        },
        BuiltFirstValueTestCase {
            description: "a letter outside Latin-1 raises",
            sql: path(r#"{"λ": 1}"#, false, "TRUE"),
            expected_outcome: Err("a character no recording shows"),
        },
        BuiltFirstValueTestCase {
            description: "a key with a backslash raises",
            sql: path(r#"{"a\\\\b": 1}"#, true, "TRUE"),
            expected_outcome: Err("a character no recording shows"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_first_outcome(&test_case.sql, test_case.expected_outcome),
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
fn given_misspelled_text_when_trying_a_month_name_format_then_it_is_null_and_spaces_are_trimmed() {
    let test_cases = [
        FirstValueTestCase {
            description: "surrounding spaces are trimmed",
            sql: "SELECT TO_VARCHAR(TRY_TO_TIMESTAMP_NTZ(' 01-MAR-26 10:15 ', 'DD-MON-YY HH24:MI')) AS v",
            expected_outcome: Ok("2026-03-01 10:15:00.000"),
        },
        FirstValueTestCase {
            description: "seconds the format does not read make it NULL",
            sql: "SELECT TRY_TO_TIMESTAMP_NTZ('01-MAR-26 10:15:30', 'DD-MON-YY HH24:MI') IS NULL AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "an ISO spelling is NULL under the month-name format",
            sql: "SELECT TRY_TO_TIMESTAMP_NTZ('2026-03-01T10:15:00Z', 'DD-MON-YY HH24:MI') IS NULL AS v",
            expected_outcome: Ok("true"),
        },
        FirstValueTestCase {
            description: "a tab, which no recording shows, still raises",
            sql: "SELECT TRY_TO_TIMESTAMP_NTZ('01-MAR-26\t10:15', 'DD-MON-YY HH24:MI') IS NULL AS v",
            expected_outcome: Err("date_text_spelling"),
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
fn given_a_rounding_mode_when_converting_round_then_half_to_even_rounds_ties_to_even_digits() {
    let test_cases = [
        FirstValueTestCase {
            description: "HALF_TO_EVEN rounds a tie down to an even digit",
            sql: "SELECT ROUND(2.25, 1, 'HALF_TO_EVEN') AS v",
            expected_outcome: Ok("2.2"),
        },
        FirstValueTestCase {
            description: "HALF_TO_EVEN rounds a tie up to an even digit, also below zero",
            sql: "SELECT ROUND(-2.35, 1, 'HALF_TO_EVEN') AS v",
            expected_outcome: Ok("-2.4"),
        },
        FirstValueTestCase {
            description: "a value past the tie rounds to the nearest",
            sql: "SELECT ROUND(2.2501, 1, 'HALF_TO_EVEN') AS v",
            expected_outcome: Ok("2.3"),
        },
        FirstValueTestCase {
            description: "negative places round a tie of tens to even",
            sql: "SELECT ROUND(25, -1, 'HALF_TO_EVEN') AS v",
            expected_outcome: Ok("20"),
        },
        FirstValueTestCase {
            description: "HALF_AWAY_FROM_ZERO is ROUND without a mode",
            sql: "SELECT ROUND(2.25, 1, 'HALF_AWAY_FROM_ZERO') AS v",
            expected_outcome: Ok("2.3"),
        },
        FirstValueTestCase {
            description: "a FLOAT with a mode refuses",
            sql: "SELECT ROUND(2.25::FLOAT, 1, 'HALF_TO_EVEN') AS v",
            expected_outcome: Err("FLOAT with a rounding mode"),
        },
        FirstValueTestCase {
            description: "a mode spelled otherwise refuses",
            sql: "SELECT ROUND(2.25, 1, 'half_to_even') AS v",
            expected_outcome: Err("rounding mode no recording shows"),
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
fn given_a_frame_when_converting_avg_over_a_window_then_it_averages_the_frame_values_exactly() {
    let rows = "(VALUES (1, 'a', 1), (2, 'a', NULL), (3, 'a', 2), (4, 'a', 2), (5, 'b', NULL)) AS t(n, g, x)";
    let averaged = |over: &str, n: u8| {
        format!(
            "SELECT COALESCE(TO_VARCHAR(a), 'null') AS v FROM (SELECT n, AVG(x) OVER ({over}) AS a \
             FROM {rows}) WHERE n = {n}"
        )
    };
    let test_cases = [
        BuiltFirstValueTestCase {
            description: "a ROWS frame skips NULLs and keeps three more decimal places",
            sql: averaged(
                "PARTITION BY g ORDER BY n ROWS BETWEEN 2 PRECEDING AND CURRENT ROW",
                3,
            ),
            expected_outcome: Ok("1.500"),
        },
        BuiltFirstValueTestCase {
            description: "a ROWS frame truncates past its third place, as recorded",
            sql: averaged(
                "PARTITION BY g ORDER BY n ROWS BETWEEN 3 PRECEDING AND CURRENT ROW",
                4,
            ),
            expected_outcome: Ok("1.666"),
        },
        BuiltFirstValueTestCase {
            description: "a frame without values is NULL",
            sql: averaged(
                "PARTITION BY g ORDER BY n ROWS BETWEEN 2 PRECEDING AND 1 PRECEDING",
                1,
            ),
            expected_outcome: Ok("null"),
        },
        BuiltFirstValueTestCase {
            description: "a partition of NULLs is NULL over the whole partition",
            sql: averaged("PARTITION BY g", 5),
            expected_outcome: Ok("null"),
        },
        BuiltFirstValueTestCase {
            description: "a whole unordered partition truncates as a frame does",
            sql: averaged("PARTITION BY g", 4),
            expected_outcome: Ok("1.666"),
        },
        BuiltFirstValueTestCase {
            description: "a negative average truncates toward zero",
            sql: "SELECT TO_VARCHAR(AVG(x) OVER (ORDER BY n ROWS BETWEEN 2 PRECEDING AND CURRENT ROW)) \
                  AS v FROM (VALUES (1, -2), (2, 0), (3, 0)) AS t(n, x) QUALIFY n = 3"
                .to_owned(),
            expected_outcome: Ok("-0.666"),
        },
        BuiltFirstValueTestCase {
            description: "an ordered window without a frame runs to the current row",
            sql: averaged("PARTITION BY g ORDER BY n", 2),
            expected_outcome: Ok("1.000000"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_first_outcome(&test_case.sql, test_case.expected_outcome),
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
fn given_system_typeof_with_its_suffix_removed_when_converting_then_the_static_type_is_named() {
    let stripped = |operand: &str| {
        format!("SELECT REGEXP_REPLACE(SYSTEM$TYPEOF({operand}), '\\\\[.*\\\\]$', '') AS v")
    };
    let test_cases = [
        BuiltFirstValueTestCase {
            description: "a TIME cast is TIME(9)",
            sql: stripped("'13:45:01'::TIME"),
            expected_outcome: Ok("TIME(9)"),
        },
        BuiltFirstValueTestCase {
            description: "CURRENT_DATE is DATE",
            sql: stripped("CURRENT_DATE()"),
            expected_outcome: Ok("DATE"),
        },
        BuiltFirstValueTestCase {
            description: "a NUMBER(18,2) minus NULL keeps a widened NUMBER",
            sql: stripped("1.50::NUMBER(18,2) - NULL"),
            expected_outcome: Ok("NUMBER(19,2)"),
        },
        BuiltFirstValueTestCase {
            description: "bare SYSTEM$TYPEOF refuses, as its suffix depends on the plan",
            sql: "SELECT SYSTEM$TYPEOF(1) AS v".to_owned(),
            expected_outcome: Err("storage suffix"),
        },
        BuiltFirstValueTestCase {
            description: "text refuses, as its length is not tracked",
            sql: stripped("'x'"),
            expected_outcome: Err("length or kind is not tracked"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_first_outcome(&test_case.sql, test_case.expected_outcome),
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
fn given_urls_when_converting_parse_url_then_parts_and_parameters_are_read_as_recorded() {
    let read = |url: &str, path: &str| {
        format!("SELECT COALESCE(PARSE_URL('{url}', 1){path}::VARCHAR, 'null') AS v")
    };
    let test_cases = [
        BuiltFirstValueTestCase {
            description: "the host keeps its case",
            sql: read("https://Shop.Example.com:8443/a/b?x=1#top", "['host']"),
            expected_outcome: Ok("Shop.Example.com"),
        },
        BuiltFirstValueTestCase {
            description: "the last of a repeated parameter is kept, undecoded",
            sql: read("https://example.com/p?s=a&s=b%20c", "['parameters']['s']"),
            expected_outcome: Ok("b%20c"),
        },
        BuiltFirstValueTestCase {
            description: "a query after the fragment mark belongs to the fragment",
            sql: read("https://example.com/p#frag?x=1", "['fragment']"),
            expected_outcome: Ok("frag?x=1"),
        },
        BuiltFirstValueTestCase {
            description: "a trailing slash is an empty path",
            sql: read("https://example.com/", "['path']"),
            expected_outcome: Ok(""),
        },
        BuiltFirstValueTestCase {
            description: "text without a scheme is the error object",
            sql: read("example.com/landing", "['error']"),
            expected_outcome: Ok("scheme not specified"),
        },
        BuiltFirstValueTestCase {
            description: "a URL with user information raises",
            sql: read("https://user@example.com/", "['host']"),
            expected_outcome: Err("a URL in a shape no recording shows"),
        },
        BuiltFirstValueTestCase {
            description: "the strict mode refuses",
            sql: "SELECT PARSE_URL('https://example.com/')::VARCHAR AS v".to_owned(),
            expected_outcome: Err("permissive mode 1"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_first_outcome(&test_case.sql, test_case.expected_outcome),
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
fn given_nvl2_when_converting_then_the_branch_follows_whether_the_first_is_null_or_it_refuses() {
    let test_cases = [
        FirstValueTestCase {
            description: "the second argument where the first is not NULL",
            sql: "SELECT NVL2(1, 'set', 'unset') AS v",
            expected_outcome: Ok("set"),
        },
        FirstValueTestCase {
            description: "the third argument where the first is NULL",
            sql: "SELECT NVL2(NULL, 'set', 'unset') AS v",
            expected_outcome: Ok("unset"),
        },
        FirstValueTestCase {
            description: "a NUMBER and a FLOAT branch unify to FLOAT",
            sql: "SELECT NVL2(NULL, 1, 2.5::FLOAT) AS v",
            expected_outcome: Ok("2.5"),
        },
        FirstValueTestCase {
            description: "text beside a NUMBER, whose reading no recording shows",
            sql: "SELECT NVL2(1, '7', 8) AS v",
            expected_outcome: Err("no proven common type"),
        },
        FirstValueTestCase {
            description: "a semi-structured first argument",
            sql: "SELECT NVL2(PARSE_JSON('null'), 1, 2) AS v",
            expected_outcome: Err("semi-structured"),
        },
        FirstValueTestCase {
            description: "four arguments, which Snowflake rejects",
            sql: "SELECT NVL2(1, 2, 3, 4) AS v",
            expected_outcome: Err("NVL2"),
        },
        FirstValueTestCase {
            description: "two arguments, which Snowflake rejects",
            sql: "SELECT NVL2(1, 2) AS v",
            expected_outcome: Err("NVL2"),
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
fn given_each_documented_format_element_when_writing_text_then_it_converts_as_recorded_or_refuses()
{
    let test_cases = [
        FirstValueTestCase {
            description: "the four-digit year",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'YYYY') AS v",
            expected_outcome: Ok("2024"),
        },
        FirstValueTestCase {
            description: "the zero-padded month",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MM') AS v",
            expected_outcome: Ok("09"),
        },
        FirstValueTestCase {
            description: "the zero-padded day",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'DD') AS v",
            expected_outcome: Ok("05"),
        },
        FirstValueTestCase {
            description: "MON, the month's abbreviation in title case",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MON') AS v",
            expected_outcome: Ok("Sep"),
        },
        FirstValueTestCase {
            description: "MMMM, the month's full name, not MM twice",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MMMM') AS v",
            expected_outcome: Ok("September"),
        },
        FirstValueTestCase {
            description: "MON between other fields",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'DD MON YYYY') AS v",
            expected_outcome: Ok("05 Sep 2024"),
        },
        FirstValueTestCase {
            description: "adjacent numeric fields",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'YYYYMMDD') AS v",
            expected_outcome: Ok("20240905"),
        },
        FirstValueTestCase {
            description: "the 24-hour hour",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'HH24') AS v",
            expected_outcome: Ok("13"),
        },
        FirstValueTestCase {
            description: "the minutes",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'MI') AS v",
            expected_outcome: Ok("04"),
        },
        FirstValueTestCase {
            description: "the seconds",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'SS') AS v",
            expected_outcome: Ok("05"),
        },
        FirstValueTestCase {
            description: "milliseconds",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'FF3') AS v",
            expected_outcome: Ok("123"),
        },
        FirstValueTestCase {
            description: "FF at the timestamp's precision",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'FF') AS v",
            expected_outcome: Ok("123"),
        },
        FirstValueTestCase {
            description: "MON of a timestamp",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'MON') AS v",
            expected_outcome: Ok("Sep"),
        },
        FirstValueTestCase {
            description: "MON in lower case, whose letter case no recording shows",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'mon') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "MMMM in mixed case",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'Mmmm') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "YY",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'YY') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "D",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'D') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "DY",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'DY') AS v",
            expected_outcome: Ok("Thu"),
        },
        FirstValueTestCase {
            description: "UUUU",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'UUUU') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "HH12",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'HH12') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "HH",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'HH') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "H24",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'H24') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "H12",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'H12') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "H",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'H') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "AM",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'HH12 AM') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "PM",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'HH12 PM') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "A",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'HH12 A') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "P",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'HH12 P') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "FF0",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'FF0') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "FF1",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'FF1') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "FF7",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'FF7') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "TZH:TZM",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'TZH:TZM') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "TZHTZM",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'TZHTZM') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "TZH",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'TZH') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "TZM",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'TZM') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "MMMMMM, which MMMM and MM would spell",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MMMMMM') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "MM then MON, a run of M",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MMMON') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "DDDD, which DD would match twice",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'DDDD') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "YYYYYYYY, which YYYY would match twice",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'YYYYYYYY') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "SSSS, which SS would match twice",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'SSSS') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "MMMM directly before DD, as recorded",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MMMMDD') AS v",
            expected_outcome: Ok("September05"),
        },
        FirstValueTestCase {
            description: "MONMM, which Snowflake writes as the full name, not MON then MM",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MONMM') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "MON directly before DD, unrecorded",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MONDD') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "bare FF directly after MON",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'MONFF') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "MMMM directly before YYYY, unrecorded",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MMMMYYYY') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "bare FF directly after MMMM, unrecorded",
            sql: "SELECT TO_CHAR('2024-09-05 13:04:05.123'::TIMESTAMP_NTZ(3), 'MMMMFF') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "MON directly after DD, unrecorded",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'DDMON') AS v",
            expected_outcome: Err("which no recording shows"),
        },
        FirstValueTestCase {
            description: "MON after a separator still converts",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'MON-MM') AS v",
            expected_outcome: Ok("Sep-09"),
        },
        FirstValueTestCase {
            description: "time fields of a DATE",
            sql: "SELECT TO_CHAR(DATE '2024-09-05', 'HH24 MI') AS v",
            expected_outcome: Err("which no recording shows"),
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
fn given_unary_signs_when_converting_then_they_type_and_refuse_as_snowflake_does() {
    let typed = |operand: &str| {
        format!("SELECT REGEXP_REPLACE(SYSTEM$TYPEOF({operand}), '\\\\[.*\\\\]$', '') AS v")
    };
    let test_cases = [
        BuiltFirstValueTestCase {
            description: "a plus gives a one-digit literal a second integer digit",
            sql: typed("+1"),
            expected_outcome: Ok("NUMBER(2,0)"),
        },
        BuiltFirstValueTestCase {
            description: "a spaced plus over parentheses types the same",
            sql: typed("+ (1)"),
            expected_outcome: Ok("NUMBER(2,0)"),
        },
        BuiltFirstValueTestCase {
            description: "a plus leaves a literal with two or more integer digits alone",
            sql: typed("+123"),
            expected_outcome: Ok("NUMBER(3,0)"),
        },
        BuiltFirstValueTestCase {
            description: "a plus keeps the scale of a decimal it widens",
            sql: typed("+0.25"),
            expected_outcome: Ok("NUMBER(4,2)"),
        },
        BuiltFirstValueTestCase {
            description: "a plus over a parenthesised minus widens the minus",
            sql: typed("+(-1)"),
            expected_outcome: Ok("NUMBER(2,0)"),
        },
        BuiltFirstValueTestCase {
            description: "a minus over a parenthesised minus keeps its type",
            sql: typed("-(-1)"),
            expected_outcome: Ok("NUMBER(1,0)"),
        },
        BuiltFirstValueTestCase {
            description: "a plus keeps a NUMBER(38,37) within 38 digits",
            sql: typed("+0.1234567890123456789012345678901234567"),
            expected_outcome: Ok("NUMBER(38,37)"),
        },
        BuiltFirstValueTestCase {
            description: "a widened plus keeps its value",
            sql: "SELECT +(1.5) * 2 AS v".to_owned(),
            expected_outcome: Ok("3.0"),
        },
        BuiltFirstValueTestCase {
            description: "two plus signs, which Snowflake rejects (1005)",
            sql: "SELECT ++1 AS v".to_owned(),
            expected_outcome: Err("two adjacent signs"),
        },
        BuiltFirstValueTestCase {
            description: "a spaced minus over a plus",
            sql: "SELECT - +1 AS v".to_owned(),
            expected_outcome: Err("two adjacent signs"),
        },
        BuiltFirstValueTestCase {
            description: "two spaced minus signs",
            sql: "SELECT - -1 AS v".to_owned(),
            expected_outcome: Err("two adjacent signs"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            shop_first_outcome(&test_case.sql, test_case.expected_outcome),
            test_case
                .expected_outcome
                .map(str::to_owned)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
