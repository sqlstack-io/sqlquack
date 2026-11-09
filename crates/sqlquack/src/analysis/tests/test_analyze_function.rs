use crate::analysis::tests::helpers::{
    event_results, event_results_allowing, function_body_results,
};
use crate::analysis::tests::test_types::{EventResultTestCase, FunctionBodyTestCase};

#[test]
fn given_a_sql_function_body_when_converting_then_its_duckdb_macro_computes_the_declared_values() {
    let test_cases = [
        FunctionBodyTestCase {
            description: "a NUMBER scalar body of exactly the declared type converts",
            name: "__SQLQUACK_UDF__ROUNDED",
            body: "CAST(amount * 2 AS NUMBER(10,2))",
            call: "SELECT CAST(\"__SQLQUACK_UDF__ROUNDED\"(1.25) AS VARCHAR)",
            expected_values: vec!["2.50".to_owned()],
        },
        FunctionBodyTestCase {
            description: "a table body reads its parameter and the schema's relations",
            name: "LARGE_ORDERS",
            body: "SELECT amount FROM orders WHERE amount >= minimum",
            call: "SELECT CAST(\"AMOUNT\" AS VARCHAR) FROM \"LARGE_ORDERS\"(10)",
            expected_values: vec!["12.50".to_owned()],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            function_body_results(test_case.name, test_case.body, test_case.call),
            test_case.expected_values,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_names_shared_with_lambda_parameters_when_running_the_conversion_then_values_are_kept() {
    let test_cases = [
        EventResultTestCase {
            description: "a VARIANT result column named like a lambda parameter keeps its value",
            sql: "SELECT payload:a AS r FROM events",
            payload: "{\"a\":{\"k\":1}}",
            expected_values: vec!["{\"k\":1}".to_owned()],
        },
        EventResultTestCase {
            description: "a table alias named like a lambda parameter does not shadow it",
            sql: "SELECT OBJECT_DELETE(e.payload, 'a') AS o FROM events AS e",
            payload: "{\"a\":1,\"b\":2}",
            expected_values: vec!["{\"b\":2}".to_owned()],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            event_results(test_case.sql, test_case.payload),
            test_case.expected_values,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_hash_when_running_the_conversion_then_equal_values_hash_equally_and_order_matters() {
    let test_cases = [
        EventResultTestCase {
            description: "equal objects written with keys in byte order hash equally",
            sql: "SELECT HASH(payload:a) = HASH(payload:b) AS same FROM events",
            payload: "{\"a\":{\"k\":1,\"m\":[2]},\"b\":{\"k\":1,\"m\":[2]}}",
            expected_values: vec!["true".to_owned()],
        },
        EventResultTestCase {
            description: "integers of different precisions hash by value",
            sql: "SELECT HASH(CAST(7 AS NUMBER(5,0))) = HASH(CAST(7 AS NUMBER(38,0))) AS same FROM events",
            payload: "{}",
            expected_values: vec!["true".to_owned()],
        },
        EventResultTestCase {
            description: "the arguments' order changes the hash, and NULL hashes to a value",
            sql: "SELECT HASH('a', 'b') <> HASH('b', 'a') AND HASH(NULL) IS NOT NULL AS ordered FROM events",
            payload: "{}",
            expected_values: vec!["true".to_owned()],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            event_results_allowing(test_case.sql, test_case.payload, &["hash"]),
            test_case.expected_values,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_editdistance_under_aliases_named_like_its_helpers_when_running_then_values_are_exact() {
    let test_cases = [
        EventResultTestCase {
            description: "a relation aliased p with columns l and r does not shadow the operands",
            sql: "SELECT EDITDISTANCE(p.a, p.b) AS d FROM (SELECT payload:a::VARCHAR AS a, \
                  payload:b::VARCHAR AS b, 'zzzzzzzzzzzz' AS l, 'q' AS r FROM events) AS p",
            payload: "{\"a\":\"kitten\",\"b\":\"sitting\"}",
            expected_values: vec!["3".to_owned()],
        },
        EventResultTestCase {
            description: "equal texts stay at distance 0 under the same alias",
            sql: "SELECT EDITDISTANCE(p.a, 'kitten') AS same FROM (SELECT payload:a::VARCHAR AS a, \
                  'zzzzzzzzzzzz' AS l, 'q' AS r FROM events) AS p",
            payload: "{\"a\":\"kitten\",\"b\":\"sitting\"}",
            expected_values: vec!["0".to_owned()],
        },
        EventResultTestCase {
            description: "an alias x with columns c and i does not shadow the character steps",
            sql: "SELECT EDITDISTANCE(x.a, x.b) AS d FROM (SELECT payload:a::VARCHAR AS a, \
                  payload:b::VARCHAR AS b, 'w' AS c, 5 AS i FROM events) AS x",
            payload: "{\"a\":\"kitten\",\"b\":\"sitting\"}",
            expected_values: vec!["3".to_owned()],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            event_results(test_case.sql, test_case.payload),
            test_case.expected_values,
            "{}",
            test_case.description
        );
    }
}
