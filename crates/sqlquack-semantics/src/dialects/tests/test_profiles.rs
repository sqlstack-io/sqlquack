use crate::dialects::constants::{BIGQUERY, DUCKDB, POSTGRES, SNOWFLAKE, SQLITE};
use crate::dialects::main::identifier_name::identifier_name;
use crate::dialects::tests::test_types::IdentifierTestCase;

#[test]
fn given_identifiers_when_resolving_by_profile_then_each_dialect_folds_and_compares_its_way() {
    let test_cases = [
        IdentifierTestCase {
            description: "Snowflake resolves an unquoted identifier to upper case",
            profile: &SNOWFLAKE,
            written: ("customer_id", false),
            compared_with: "CUSTOMER_ID",
            expected_resolved: "CUSTOMER_ID",
            expected_same: true,
        },
        IdentifierTestCase {
            description: "Snowflake keeps a quoted identifier as written and compares it exactly",
            profile: &SNOWFLAKE,
            written: ("customer_id", true),
            compared_with: "CUSTOMER_ID",
            expected_resolved: "customer_id",
            expected_same: false,
        },
        IdentifierTestCase {
            description: "DuckDB keeps an identifier as written and compares it without case",
            profile: &DUCKDB,
            written: ("customer_id", false),
            compared_with: "CUSTOMER_ID",
            expected_resolved: "customer_id",
            expected_same: true,
        },
        IdentifierTestCase {
            description: "PostgreSQL resolves an unquoted identifier to lower case",
            profile: &POSTGRES,
            written: ("Customer_ID", false),
            compared_with: "customer_id",
            expected_resolved: "customer_id",
            expected_same: true,
        },
        IdentifierTestCase {
            description: "PostgreSQL keeps a quoted identifier as written and compares it exactly",
            profile: &POSTGRES,
            written: ("Customer_ID", true),
            compared_with: "customer_id",
            expected_resolved: "Customer_ID",
            expected_same: false,
        },
        IdentifierTestCase {
            description: "BigQuery resolves a quoted identifier to lower case and compares it without case",
            profile: &BIGQUERY,
            written: ("Customer_ID", true),
            compared_with: "CUSTOMER_id",
            expected_resolved: "customer_id",
            expected_same: true,
        },
        IdentifierTestCase {
            description: "SQLite keeps an identifier as written and compares it without case",
            profile: &SQLITE,
            written: ("Customer_ID", true),
            compared_with: "customer_id",
            expected_resolved: "Customer_ID",
            expected_same: true,
        },
    ];
    for test_case in test_cases {
        let (name, quoted) = test_case.written;
        let node = serde_json::json!({ "name": name, "quoted": quoted });
        let resolved = identifier_name(&node, test_case.profile).unwrap_or_default();
        assert_eq!(
            resolved, test_case.expected_resolved,
            "{}",
            test_case.description
        );
        assert_eq!(
            test_case
                .profile
                .same_identifier(&resolved, test_case.compared_with),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}
