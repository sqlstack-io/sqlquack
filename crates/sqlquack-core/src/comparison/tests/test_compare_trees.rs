use polyglot_sql::Expression;

use crate::comparison::tests::helpers::{
    difference_path, field_present, parse_one, raw_trees_equal, sql_difference_path, typed_column,
    untested_fields,
};
use crate::comparison::tests::test_types::{
    DeclarationTestCase, DifferenceTestCase, SpellingTestCase, TreeTestCase,
};

#[test]
fn given_spellings_differing_only_in_a_declared_field_when_comparing_then_trees_match() {
    let test_cases = [
        SpellingTestCase {
            description: "whitespace moves source positions",
            field: "span",
            left: "SELECT order_id FROM orders",
            right: "SELECT   order_id   FROM orders",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
        SpellingTestCase {
            description: "a comment",
            field: "comments",
            left: "SELECT order_id /* the key */ FROM orders",
            right: "SELECT order_id FROM orders",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
        SpellingTestCase {
            description: "`::` and CAST",
            field: "double_colon_syntax",
            left: "SELECT amount::INT FROM orders",
            right: "SELECT CAST(amount AS INT) FROM orders",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
        SpellingTestCase {
            description: "an alias with and without AS",
            field: "alias_explicit_as",
            left: "SELECT amount AS total FROM orders",
            right: "SELECT amount total FROM orders",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
        SpellingTestCase {
            description: "the letter case of AS",
            field: "alias_keyword",
            left: "SELECT * FROM (SELECT 1 AS n) as numbers",
            right: "SELECT * FROM (SELECT 1 AS n) AS numbers",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
        SpellingTestCase {
            description: "JOIN and INNER JOIN",
            field: "use_inner_keyword",
            left: "SELECT * FROM orders o INNER JOIN customers c ON o.customer_id = c.customer_id",
            right: "SELECT * FROM orders o JOIN customers c ON o.customer_id = c.customer_id",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
        SpellingTestCase {
            description: "LEFT JOIN and LEFT OUTER JOIN",
            field: "use_outer_keyword",
            left: "SELECT * FROM orders o LEFT OUTER JOIN customers c \
                   ON o.customer_id = c.customer_id",
            right: "SELECT * FROM orders o LEFT JOIN customers c ON o.customer_id = c.customer_id",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
        SpellingTestCase {
            description: "the name a typed function node was written with",
            field: "original_name",
            left: "SELECT count(sku) FROM products",
            right: "SELECT COUNT(sku) FROM products",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
        SpellingTestCase {
            description: "the letter case of a generic function name",
            field: "name",
            left: "SELECT my_lookup(sku) FROM products",
            right: "SELECT MY_LOOKUP(sku) FROM products",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
        SpellingTestCase {
            description: "a dollar-quoted string and a quoted string",
            field: "literal_type",
            left: "SELECT $$fragile$$ FROM products",
            right: "SELECT 'fragile' FROM products",
            expected_raw_trees_equal: false,
            expected_field_present: true,
            expected_difference_path: None,
        },
    ];

    for test_case in test_cases {
        let left = parse_one(test_case.left);
        let right = parse_one(test_case.right);
        assert_eq!(
            raw_trees_equal(&left, &right),
            test_case.expected_raw_trees_equal,
            "{}",
            test_case.description
        );
        assert_eq!(
            field_present(&left, &right, test_case.field),
            test_case.expected_field_present,
            "{}",
            test_case.description
        );
        assert_eq!(
            difference_path(&left, &right),
            test_case.expected_difference_path,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_declared_spelling_only_fields_when_listing_tests_then_each_field_has_one() {
    let test_cases = [DeclarationTestCase {
        description: "every declared field has a spelling test above or below",
        tested_fields: &[
            "span",
            "inferred_type",
            "comments",
            "double_colon_syntax",
            "alias_explicit_as",
            "alias_keyword",
            "use_inner_keyword",
            "use_outer_keyword",
            "original_name",
        ],
        expected_untested_fields: Vec::new(),
    }];

    for test_case in test_cases {
        assert_eq!(
            untested_fields(test_case.tested_fields),
            test_case.expected_untested_fields,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_type_annotation_when_comparing_then_inferred_type_is_ignored() {
    let test_cases = [TreeTestCase {
        description: "a column with and without an inferred type",
        left: Expression::column("order_id"),
        right: typed_column("order_id"),
        expected_raw_trees_equal: false,
        expected_difference_path: None,
    }];

    for test_case in test_cases {
        assert_eq!(
            raw_trees_equal(&test_case.left, &test_case.right),
            test_case.expected_raw_trees_equal,
            "{}",
            test_case.description
        );
        assert_eq!(
            difference_path(&test_case.left, &test_case.right),
            test_case.expected_difference_path,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_trees_with_different_meaning_when_comparing_then_reports_first_differing_path() {
    let test_cases = [
        DifferenceTestCase {
            description: "a different column",
            left: "SELECT order_id FROM orders",
            right: "SELECT customer_id FROM orders",
            expected_difference_path: Some(".select.expressions[0].column.name.name".to_owned()),
        },
        DifferenceTestCase {
            description: "a different literal",
            left: "SELECT 'shipped' FROM orders",
            right: "SELECT 'pending' FROM orders",
            expected_difference_path: Some(".select.expressions[0].literal.value".to_owned()),
        },
        DifferenceTestCase {
            description: "a quoted identifier keeps its case",
            left: "SELECT \"Status\" FROM orders",
            right: "SELECT \"STATUS\" FROM orders",
            expected_difference_path: Some(".select.expressions[0].column.name.name".to_owned()),
        },
        DifferenceTestCase {
            description: "a quoted function name keeps its case",
            left: "SELECT \"Lookup\"(sku) FROM products",
            right: "SELECT \"LOOKUP\"(sku) FROM products",
            expected_difference_path: Some(".select.expressions[0].function.name".to_owned()),
        },
        DifferenceTestCase {
            description: "an extra projection",
            left: "SELECT order_id FROM orders",
            right: "SELECT order_id, status FROM orders",
            expected_difference_path: Some(".select.expressions[1]".to_owned()),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            sql_difference_path(test_case.left, test_case.right),
            test_case.expected_difference_path,
            "{}",
            test_case.description
        );
    }
}
