use crate::analysis::tests::helpers::shop_first_outcome;
use crate::analysis::tests::test_types::FirstValueTestCase;

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
            description: "a timestamp plus hours crosses midnight",
            sql: "SELECT ('2024-02-28 23:30:00'::TIMESTAMP_NTZ + INTERVAL '2 hours')::VARCHAR AS v",
            expected_outcome: Ok("2024-02-29 01:30:00.000"),
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
            sql: "SELECT POWER(2, 3) AS v",
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
            expected_outcome: Err("whose boundaries no recording places"),
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
