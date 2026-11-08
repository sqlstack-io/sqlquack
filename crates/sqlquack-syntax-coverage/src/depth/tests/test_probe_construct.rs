use crate::check::models::{LocalFailure, LocalVerdict};
use crate::depth::main::probe_construct::probe_construct;
use crate::depth::main::probed_constructs::probed_constructs;
use crate::depth::tests::helpers::{PARENTHESES, accepted, by_length, failed};
use crate::depth::tests::test_types::{ConstructsTestCase, ProbeTestCase};
use crate::formats::models::LocalParser;

#[test]
fn given_a_parser_that_fails_past_some_depth_when_probing_then_the_first_failing_depth_is_named() {
    let test_cases = [
        ProbeTestCase {
            description: "a crash from depth 1,000 stops the ladder there",
            threshold: 2_000,
            deep: failed(LocalFailure::Crash),
            shallow: LocalVerdict::Rejected,
            expected_failed_at: Some((1_000, LocalFailure::Crash)),
            expected_parses: 3,
        },
        ProbeTestCase {
            description: "a hang at the shallowest depth",
            threshold: 0,
            deep: failed(LocalFailure::Timeout),
            shallow: LocalVerdict::Rejected,
            expected_failed_at: Some((50, LocalFailure::Timeout)),
            expected_parses: 1,
        },
        ProbeTestCase {
            description: "a panic counts as a failure",
            threshold: 10_000,
            deep: failed(LocalFailure::Panic),
            shallow: accepted(),
            expected_failed_at: Some((10_000, LocalFailure::Panic)),
            expected_parses: 5,
        },
        ProbeTestCase {
            description: "a complexity guard's refusal ends cleanly at every depth",
            threshold: 0,
            deep: failed(LocalFailure::Guard),
            shallow: accepted(),
            expected_failed_at: None,
            expected_parses: 6,
        },
        ProbeTestCase {
            description: "acceptance below a depth refusal ends cleanly",
            threshold: 500,
            deep: LocalVerdict::Rejected,
            shallow: accepted(),
            expected_failed_at: None,
            expected_parses: 6,
        },
    ];
    for test_case in test_cases {
        let mut parses: usize = 0;
        let result = probe_construct(LocalParser::Postgres, &PARENTHESES, &mut |sql| {
            parses += 1;
            by_length(
                test_case.threshold,
                &test_case.deep,
                &test_case.shallow,
                sql,
            )
        });
        assert_eq!(
            (result.failed_at, parses),
            (test_case.expected_failed_at, test_case.expected_parses),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_parser_without_a_syntax_when_listing_constructs_then_only_its_own_are_probed() {
    let test_cases = [
        ConstructsTestCase {
            description: "Snowflake has ARRAY( types to mix with [] suffixes",
            parser: LocalParser::Snowflake,
            construct: "type_constructor_and_suffix",
            expected_probed: true,
        },
        ConstructsTestCase {
            description: "Postgres has no type constructor",
            parser: LocalParser::Postgres,
            construct: "type_constructor_and_suffix",
            expected_probed: false,
        },
        ConstructsTestCase {
            description: "BigQuery chains pipe operators",
            parser: LocalParser::BigQuery,
            construct: "nested_pipe_chain",
            expected_probed: true,
        },
        ConstructsTestCase {
            description: "Databricks chains pipe operators",
            parser: LocalParser::Databricks,
            construct: "nested_pipe_chain",
            expected_probed: true,
        },
        ConstructsTestCase {
            description: "Postgres has no pipe operator",
            parser: LocalParser::Postgres,
            construct: "nested_pipe_chain",
            expected_probed: false,
        },
        ConstructsTestCase {
            description: "SQL Server nests IF statements",
            parser: LocalParser::SqlServer,
            construct: "nested_if",
            expected_probed: true,
        },
        ConstructsTestCase {
            description: "SQL Server has no array type suffix",
            parser: LocalParser::SqlServer,
            construct: "type_suffix",
            expected_probed: false,
        },
        ConstructsTestCase {
            description: "every parser nests FROM parentheses",
            parser: LocalParser::Postgres,
            construct: "from_parentheses",
            expected_probed: true,
        },
    ];
    for test_case in test_cases {
        let probed: bool = probed_constructs(test_case.parser)
            .iter()
            .any(|construct| construct.name == test_case.construct);
        assert_eq!(
            probed, test_case.expected_probed,
            "{}",
            test_case.description
        );
    }
}
