use crate::dialects::constants::{BIGQUERY, DATABRICKS, DUCKDB, POSTGRES, SNOWFLAKE, SQLITE};
use crate::dialects::main::identifier_name::identifier_name;
use crate::dialects::tests::test_types::{IdentifierTestCase, TypeComparisonTestCase};
use crate::dialects::types::DialectProfile;

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
        IdentifierTestCase {
            description: "Databricks keeps a backquoted identifier as written and compares it without case",
            profile: &DATABRICKS,
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

#[test]
fn given_databricks_type_spellings_when_compared_then_synonyms_and_decimal_defaults_match() {
    let test_cases = [
        TypeComparisonTestCase {
            description: "INTEGER is INT",
            compared: ("INTEGER", "int"),
            expected_same: true,
        },
        TypeComparisonTestCase {
            description: "LONG is BIGINT",
            compared: ("LONG", "BIGINT"),
            expected_same: true,
        },
        TypeComparisonTestCase {
            description: "DECIMAL defaults to (10,0)",
            compared: ("DECIMAL", "decimal(10, 0)"),
            expected_same: true,
        },
        TypeComparisonTestCase {
            description: "NUMERIC(p) is DECIMAL(p,0)",
            compared: ("NUMERIC(12)", "DECIMAL(12,0)"),
            expected_same: true,
        },
        TypeComparisonTestCase {
            description: "a different scale is another DECIMAL",
            compared: ("DECIMAL(10,2)", "DECIMAL(10,0)"),
            expected_same: false,
        },
        TypeComparisonTestCase {
            description: "TIMESTAMP_LTZ is TIMESTAMP",
            compared: ("TIMESTAMP_LTZ", "TIMESTAMP"),
            expected_same: true,
        },
        TypeComparisonTestCase {
            description: "TIMESTAMP_NTZ is not TIMESTAMP",
            compared: ("TIMESTAMP_NTZ", "TIMESTAMP"),
            expected_same: false,
        },
        TypeComparisonTestCase {
            description: "STRING is not VARCHAR(n)",
            compared: ("STRING", "VARCHAR(10)"),
            expected_same: false,
        },
        TypeComparisonTestCase {
            description: "BIGINT is not INT",
            compared: ("BIGINT", "INT"),
            expected_same: false,
        },
    ];
    for test_case in test_cases {
        let (left, right) = test_case.compared;
        assert_eq!(
            DATABRICKS.same_type(left, right),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}
