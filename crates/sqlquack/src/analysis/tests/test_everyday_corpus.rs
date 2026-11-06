use crate::analysis::tests::helpers::{
    EVERYDAY_CORPUS, EVERYDAY_MODELS_CORPUS, everyday_outcome_mismatches,
    everyday_snowflake_rejections_not_caught, everyday_translation_rates,
};
use crate::analysis::tests::test_types::EverydayCorpusTestCase;

#[test]
fn given_everyday_snowflake_queries_when_checked_and_converted_then_each_keeps_its_outcome() {
    let test_cases = [EverydayCorpusTestCase {
        description: "every corpora/everyday query parses, checks, converts or refuses as recorded, under the strict built-in policy and the repository policy",
        directory: EVERYDAY_CORPUS,
        expected_mismatches: Vec::new(),
        expected_rates: "287 of 436 strict, 356 of 436 repository",
        // SQL Snowflake rejects (snowflake-receipts.json) that SQLQuack neither fails to parse nor
        // flags in its checks; each should become a parse or check error.
        expected_rejections_not_caught: vec![
            "SELECT COUNT(*) FILTER (WHERE status = 'shipped') AS shipped FROM orders",
            "SELECT order_id, total FROM orders PIVOT (SUM(total) FOR status IN ('shipped', 'pending')) AS p",
            "SELECT order_id FROM orders WHERE status == 'shipped'",
            "SELECT order_id FROM orders WHERE status IN 'shipped'",
            "SELECT order_id FROM orders WHERE total >> 10",
            "SELECT order_id FROM orders WHERE ordered_at > 5",
            "SELECT LENGTH(order_id, 2) AS x FROM orders",
            "SELECT order_id FROM orders o, orders o",
        ],
    }];

    for test_case in test_cases {
        assert_eq!(
            everyday_outcome_mismatches(test_case.directory),
            test_case.expected_mismatches,
            "{}",
            test_case.description
        );
        assert_eq!(
            everyday_translation_rates(test_case.directory),
            test_case.expected_rates,
            "{}",
            test_case.description
        );
        assert_eq!(
            everyday_snowflake_rejections_not_caught(test_case.directory),
            test_case.expected_rejections_not_caught,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_dbt_style_snowflake_models_when_checked_and_converted_then_each_keeps_its_outcome() {
    let test_cases = [EverydayCorpusTestCase {
        description: "every corpora/everyday-models query parses, checks, converts or refuses as recorded, under the strict built-in policy and the repository policy",
        directory: EVERYDAY_MODELS_CORPUS,
        expected_mismatches: Vec::new(),
        expected_rates: "175 of 295 strict, 247 of 295 repository",
        // SQL Snowflake rejects that SQLQuack neither fails to parse nor flags in its checks:
        // an ambiguous column, ON after LATERAL FLATTEN, a WINDOW clause, a pivoted-away
        // column, and CHANGES without change tracking.
        expected_rejections_not_caught: vec![
            "with customers as (\n    select * from stg_customers\n),\norders as (\n    select * from stg_orders\n),\npayments as (\n    select * from stg_payments\n),\ncustomer_orders as (\n    select\n        customer_id,\n        min(order_date) as first_order,\n        max(order_date) as most_recent_order,\n        count(order_id) as number_of_orders\n    from orders\n    group by customer_id\n),\ncustomer_payments as (\n    select\n        orders.customer_id,\n        sum(amount) as total_amount\n    from payments\n    left join orders on payments.order_id = orders.order_id\n    group by orders.customer_id\n),\nfinal as (\n    select\n        customers.customer_id,\n        customers.first_name,\n        customers.last_name,\n        customer_orders.first_order,\n        customer_orders.most_recent_order,\n        customer_orders.number_of_orders,\n        customer_payments.total_amount as customer_lifetime_value\n    from customers\n    left join customer_orders on customers.customer_id = customer_orders.customer_id\n    left join customer_payments on customers.customer_id = customer_payments.customer_id\n)\nselect * from final",
            "select\n    e.event_id,\n    f.value:sku::varchar as sku\nfrom events e\ninner join lateral flatten(input => e.payload:items) f on true",
            "select\n    order_id,\n    sum(total) over w as running_total,\n    avg(total) over w as running_avg\nfrom orders\nwindow w as (partition by customer_id order by ordered_at)",
            "select customer_id, channel, total_web, total_store\nfrom (select customer_id, channel, total from orders)\npivot (sum(total) for channel in ('web' as total_web, 'store' as total_store))",
            "select *\nfrom stg_customers\nchanges (information => default)\nat (offset => -3600)",
        ],
    }];

    for test_case in test_cases {
        assert_eq!(
            everyday_outcome_mismatches(test_case.directory),
            test_case.expected_mismatches,
            "{}",
            test_case.description
        );
        assert_eq!(
            everyday_translation_rates(test_case.directory),
            test_case.expected_rates,
            "{}",
            test_case.description
        );
        assert_eq!(
            everyday_snowflake_rejections_not_caught(test_case.directory),
            test_case.expected_rejections_not_caught,
            "{}",
            test_case.description
        );
    }
}
