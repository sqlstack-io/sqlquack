use crate::fingerprint::main::query_fingerprint::query_fingerprint;
use crate::fingerprint::models::FingerprintDialect;
use crate::sqlserver::profile::tests::test_types::{DialectNameTestCase, StabilityTestCase};

#[test]
fn given_two_queries_when_fingerprinted_then_only_meaningful_edits_change_the_digest() {
    let test_cases = [
        StabilityTestCase {
            description: "SQL Server keyword case is ignored, TOP, JOIN, CASE, WHEN, END and BY included",
            dialect: FingerprintDialect::SqlServer,
            first: "SELECT TOP 5 o.id, CASE WHEN o.x > 1 THEN 'a' END AS k FROM dbo.orders AS o JOIN dbo.c AS c ON o.id = c.id ORDER BY o.id",
            second: "select top 5 o.id, case when o.x > 1 then 'a' end as k from dbo.orders as o join dbo.c as c on o.id = c.id order by o.id",
            expected_same: true,
        },
        StabilityTestCase {
            description: "SQL Server identifier case stays significant, since collation decides it",
            dialect: FingerprintDialect::SqlServer,
            first: "SELECT Id FROM orders",
            second: "SELECT id FROM orders",
            expected_same: false,
        },
        StabilityTestCase {
            description: "a SQL Server bracketed keyword is a name, so its case is significant",
            dialect: FingerprintDialect::SqlServer,
            first: "SELECT [Order] FROM t",
            second: "SELECT [order] FROM t",
            expected_same: false,
        },
    ];
    for test_case in test_cases {
        let first = query_fingerprint(test_case.first, test_case.dialect);
        let second = query_fingerprint(test_case.second, test_case.dialect);
        assert!(first.is_ok() && second.is_ok(), "{}", test_case.description);
        assert_eq!(
            first == second,
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_dialect_names_when_resolved_then_each_spelling_names_its_dialect() {
    let test_cases = [
        DialectNameTestCase {
            description: "SQL Server by SQLClean's name",
            name: "tsql",
            expected_dialect: Ok(FingerprintDialect::SqlServer),
        },
        DialectNameTestCase {
            description: "SQL Server by its other short name",
            name: "mssql",
            expected_dialect: Ok(FingerprintDialect::SqlServer),
        },
        DialectNameTestCase {
            description: "SQL Server by its own name",
            name: "sqlserver",
            expected_dialect: Ok(FingerprintDialect::SqlServer),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            FingerprintDialect::from_name(test_case.name),
            test_case.expected_dialect,
            "{}",
            test_case.description
        );
    }
}
