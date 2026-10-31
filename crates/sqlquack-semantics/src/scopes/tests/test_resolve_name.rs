use crate::scopes::models::Scope;
use crate::scopes::tests::helpers::{relation, resolved};
use crate::scopes::tests::test_types::ResolveNameTestCase;

#[test]
fn given_a_scope_when_resolving_names_then_columns_aliases_and_outer_queries_resolve_in_order() {
    let outer: Scope<'_, &'static str> =
        Scope::new(vec![relation("C", &["CUSTOMER_ID", "NOTE"])], None);
    let mut scope = Scope::new(
        vec![
            relation("O", &["ORDER_ID", "AMOUNT", "NOTE"]),
            relation("P", &["note"]),
            relation("Q", &["ORDER_ID"]),
        ],
        Some(&outer),
    );
    scope.aliases.push(("TOTAL".to_owned(), "total"));
    let test_cases = [
        ResolveNameTestCase {
            description: "an unqualified column of one relation",
            name: (None, "AMOUNT"),
            expected_outcome: Ok("O.AMOUNT"),
        },
        ResolveNameTestCase {
            description: "a qualified column",
            name: (Some("O"), "NOTE"),
            expected_outcome: Ok("O.NOTE"),
        },
        ResolveNameTestCase {
            description: "a name differing from a column only by case refuses for DuckDB",
            name: (Some("P"), "NOTE"),
            expected_outcome: Err(
                "`NOTE` matches `note` only without regard to case, which DuckDB cannot tell apart",
            ),
        },
        ResolveNameTestCase {
            description: "a name in two relations is ambiguous",
            name: (None, "ORDER_ID"),
            expected_outcome: Err("column `ORDER_ID` is ambiguous"),
        },
        ResolveNameTestCase {
            description: "a select alias resolves when no column has the name",
            name: (None, "TOTAL"),
            expected_outcome: Ok("alias total"),
        },
        ResolveNameTestCase {
            description: "an outer query's column resolves last",
            name: (None, "CUSTOMER_ID"),
            expected_outcome: Ok("C.CUSTOMER_ID"),
        },
        ResolveNameTestCase {
            description: "a name nothing has does not resolve",
            name: (Some("O"), "MISSING"),
            expected_outcome: Err("column `O.MISSING` does not resolve"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            resolved(&scope, test_case.name),
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
fn given_a_using_join_when_resolving_its_key_then_a_bare_name_reads_the_earlier_side() {
    let mut joined = relation("R", &["K", "W"]);
    joined.merged.push("K".to_owned());
    let scope: Scope<'_, &'static str> = Scope::new(vec![relation("L", &["K", "V"]), joined], None);
    let test_cases = [
        ResolveNameTestCase {
            description: "a bare USING key reads the earlier side's column",
            name: (None, "K"),
            expected_outcome: Ok("L.K"),
        },
        ResolveNameTestCase {
            description: "a qualified USING key reads its own side's column",
            name: (Some("R"), "K"),
            expected_outcome: Ok("R.K"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            resolved(&scope, test_case.name),
            test_case
                .expected_outcome
                .map(str::to_owned)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
