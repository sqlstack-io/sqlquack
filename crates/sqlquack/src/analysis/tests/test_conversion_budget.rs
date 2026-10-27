use sqlquack_core::refusals::models::RefusalCategory;
use sqlquack_rules::conversion::models::ConversionBudget;

use crate::analysis::tests::helpers::budget_refusal;
use crate::analysis::tests::test_types::BudgetTestCase;

const ORDER_NOTES: &str = "SELECT TO_VARCHAR(PARSE_JSON(note)) AS note FROM customers";

#[test]
fn given_a_conversion_budget_when_converting_then_statements_beyond_it_are_refused() {
    let test_cases = [
        BudgetTestCase {
            description: "the default budget leaves an ordinary statement converted",
            budget: ConversionBudget::default(),
            expected_category: None,
            expected_reason_fragment: "",
        },
        BudgetTestCase {
            description: "emitted SQL beyond the size budget is refused with its size",
            budget: ConversionBudget {
                size_ratio: 1,
                minimum_bytes: 0,
                ..ConversionBudget::default()
            },
            expected_category: Some(RefusalCategory::ConversionBudget),
            expected_reason_fragment: "bytes of source, beyond the budget of",
        },
        BudgetTestCase {
            description: "a minimum size keeps small statements within budget",
            budget: ConversionBudget {
                size_ratio: 1,
                ..ConversionBudget::default()
            },
            expected_category: None,
            expected_reason_fragment: "",
        },
        BudgetTestCase {
            description: "a conversion slower than the time budget is refused with its time",
            budget: ConversionBudget {
                milliseconds: 0,
                ..ConversionBudget::default()
            },
            expected_category: Some(RefusalCategory::ConversionBudget),
            expected_reason_fragment: "ms, beyond the budget of 0 ms",
        },
    ];
    for test_case in test_cases {
        let refusal = budget_refusal(ORDER_NOTES, test_case.budget);
        assert_eq!(
            refusal.as_ref().map(|(category, _)| *category),
            test_case.expected_category,
            "{}: {refusal:?}",
            test_case.description
        );
        assert!(
            refusal
                .as_ref()
                .is_none_or(|(_, reason)| reason.contains(test_case.expected_reason_fragment)),
            "{}: {refusal:?}",
            test_case.description
        );
    }
}
