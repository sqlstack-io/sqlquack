use crate::analysis::tests::helpers::{
    converts_identically, raise_places, raised_places, shop_row_count,
};
use crate::analysis::tests::test_types::{
    RaisePlacesTestCase, RaisedPlacesTestCase, ReformattedTestCase, RowCountTestCase,
};

#[test]
fn given_a_guarded_expression_repeated_in_a_grouped_query_when_converted_then_it_runs() {
    let test_cases = [
        RowCountTestCase {
            description: "a division grouped by itself",
            sql: "SELECT amount / customer_id AS r, COUNT(*) AS n FROM orders \
                  GROUP BY amount / customer_id",
            expected_rows: Ok(3),
        },
        RowCountTestCase {
            description: "TO_NUMBER grouped by itself",
            sql: "SELECT TO_NUMBER(note) AS v, COUNT(*) AS n FROM customers GROUP BY TO_NUMBER(note)",
            expected_rows: Ok(2),
        },
        RowCountTestCase {
            description: "a VARCHAR(n) cast grouped by itself",
            sql: "SELECT note::VARCHAR(1) AS v, COUNT(*) AS n FROM customers \
                  GROUP BY note::VARCHAR(1)",
            expected_rows: Ok(2),
        },
        RowCountTestCase {
            description: "a VARIANT path cast grouped by itself",
            sql: "SELECT payload:k::string AS k, COUNT(*) AS n FROM events \
                  GROUP BY payload:k::string",
            expected_rows: Ok(2),
        },
        RowCountTestCase {
            description: "HAVING on the grouped division",
            sql: "SELECT amount / customer_id AS r FROM orders GROUP BY amount / customer_id \
                  HAVING amount / customer_id > 2",
            expected_rows: Ok(2),
        },
        RowCountTestCase {
            description: "ORDER BY the grouped division",
            sql: "SELECT amount / customer_id AS r FROM orders GROUP BY amount / customer_id \
                  ORDER BY amount / customer_id",
            expected_rows: Ok(3),
        },
        RowCountTestCase {
            description: "ARRAY_AGG DISTINCT ordered by its guarded argument",
            sql: "SELECT ARRAY_AGG(DISTINCT note::VARCHAR(1)) \
                  WITHIN GROUP (ORDER BY note::VARCHAR(1)) AS v FROM customers",
            expected_rows: Ok(1),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            shop_row_count(test_case.sql),
            test_case.expected_rows,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_guarded_expressions_when_converted_then_each_raise_id_lists_its_locations() {
    let test_cases = [
        RaisePlacesTestCase {
            description: "the zero-divisor raise of a division in two places has one ID with both \
                          locations",
            sql: "SELECT amount / customer_id AS r FROM orders\nWHERE amount / customer_id > 1",
            expected_places: Ok(vec![vec![[1, 8], [2, 7]]]),
        },
        RaisePlacesTestCase {
            description: "two different divisions have IDs of their own",
            sql: "SELECT amount / customer_id AS r,\n  customer_id / amount AS s FROM orders",
            expected_places: Ok(vec![vec![[1, 8]], vec![[2, 3]]]),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            raise_places(test_case.sql),
            test_case.expected_places,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_conversion_that_raises_when_run_then_the_raise_id_resolves_to_its_expression() {
    let test_cases = [
        RaisedPlacesTestCase {
            description: "a division by zero resolves to the division",
            sql: "SELECT\n  amount / (customer_id - 1) AS ratio\nFROM orders",
            expected_places: Ok(vec![[2, 3]]),
        },
        RaisedPlacesTestCase {
            description: "a division inside a CTE resolves to the division there",
            sql: "WITH c AS (\n  SELECT customer_id, amount / (customer_id - 1) AS ratio FROM orders\n)\n\
                  SELECT ratio FROM c",
            expected_places: Ok(vec![[2, 23]]),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            raised_places(test_case.sql),
            test_case.expected_places,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_reformatted_when_converted_then_the_output_is_byte_identical() {
    let test_cases = [ReformattedTestCase {
        description: "extra lines and spaces around guarded expressions",
        texts: &[
            "SELECT customer_id / customer_id AS r, TO_NUMBER(note) AS v FROM customers \
             WHERE TO_NUMBER(note) > 1",
            "SELECT\n\n    customer_id   /   customer_id AS r,\n\tTO_NUMBER( note ) AS v\n  \
             FROM customers\n\n WHERE   TO_NUMBER(note)\n > 1",
        ],
        expected_identical: Ok(true),
    }];

    for test_case in test_cases {
        assert_eq!(
            converts_identically(test_case.texts),
            test_case.expected_identical,
            "{}",
            test_case.description
        );
    }
}
