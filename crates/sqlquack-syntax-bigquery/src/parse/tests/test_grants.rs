//! Verdicts of the BigQuery GoogleSQL parser for GRANT and REVOKE and their grantees.

use crate::parse::tests::helpers::verdict;
use crate::parse::tests::test_types::{ExpectedVerdict, VerdictTestCase};

#[test]
fn given_a_grant_or_revoke_when_parsing_then_the_verdict_is_googlesql() {
    let test_cases = [
        VerdictTestCase {
            description: "GRANT on a kinded object to string grantees is structured",
            sql: "GRANT `roles/bigquery.dataViewer` ON VIEW d.t TO 'user:a@example.com'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REVOKE takes a parameter grantee",
            sql: "REVOKE x ON EXTERNAL TABLE d.t FROM @p",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a grantee written as adjacent strings",
            sql: "GRANT `roles/bigquery.dataViewer` ON TABLE d.t TO 'user:a' '@x.com'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a grantee written as a raw string beside a plain one",
            sql: "GRANT `roles/bigquery.dataViewer` ON TABLE d.t TO r'user:a' '@x.com'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "adjacent strings as the second grantee of a list",
            sql: "GRANT x ON TABLE d.t TO 'user:b@x.com', 'user:a' '@x.com'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REVOKE from a grantee written as adjacent strings",
            sql: "REVOKE x ON TABLE d.t FROM 'user:a' '@x.com'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a grantee is no bytes literal",
            sql: "GRANT x ON d.t TO b'a'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "GRANT has no WITH GRANT OPTION",
            sql: "GRANT x ON d.t TO 'a' WITH GRANT OPTION",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an object takes at most two kind words",
            sql: "GRANT x ON a b c d.t TO 'a'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            verdict(test_case.sql),
            test_case.expected_verdict,
            "{}",
            test_case.description
        );
    }
}
