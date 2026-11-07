use crate::guard::tests::helpers::constructor_levels;
use crate::guard::tests::helpers::debug_length;
use crate::guard::tests::helpers::deep_trees;
use crate::guard::tests::helpers::guarded_parse;
use crate::guard::tests::helpers::guarded_parse_in;
use crate::guard::tests::test_types::AstDepthTestCase;
use crate::guard::tests::test_types::TreeOperationTestCase;
use crate::guard::tests::test_types::TypeDepthTestCase;
use crate::guard::tests::test_types::TYPE_DEPTHS;

#[test]
fn given_a_long_flat_operator_chain_when_parsing_then_only_real_nesting_counts_as_depth() {
    let test_cases = [
        AstDepthTestCase {
            description: "50,000 added terms are one flat chain",
            sql: || format!("SELECT {}", vec!["1"; 50_000].join(" + ")),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a long chain mixing products, sums and concatenation stays flat",
            sql: || format!("SELECT {}", vec!["2 * 3 - 1 || 'x'"; 5_000].join(" + ")),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "600 nested unary minus signs are real nesting",
            sql: || format!("SELECT {}1", "- ".repeat(600)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
    ];
    for test_case in test_cases {
        let outcome: Result<(), String> = guarded_parse(&(test_case.sql)());
        let code: Option<String> = outcome.as_ref().err().map(|error| {
            error
                .split(|c: char| !(c.is_ascii_uppercase() || c == '_'))
                .find(|word| word.starts_with("E_"))
                .unwrap_or(error)
                .to_owned()
        });
        assert_eq!(
            code.as_deref(),
            test_case.expected_error,
            "{}: {outcome:?}",
            test_case.description
        );
    }
}

#[test]
fn given_a_long_term_chain_when_parsing_on_a_2_mb_thread_then_it_parses_or_is_refused_without_overflow(
) {
    let test_cases = [
        AstDepthTestCase {
            description: "a + chain is flat and parses",
            sql: || format!("SELECT 1{}", "+1".repeat(70_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a 50,000-term + chain as a call argument parses and serializes",
            sql: || format!("SELECT f(1{}) FROM a", " + 1".repeat(50_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a 50,000-term + chain as a SUM argument parses and serializes",
            sql: || format!("SELECT SUM(1{}) FROM a", " + 1".repeat(50_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a 50,000-term + chain as a COALESCE argument parses and serializes",
            sql: || format!("SELECT COALESCE(1{}) FROM a", " + 1".repeat(50_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a 50,000-term + chain in ARRAY_CONSTRUCT parses and serializes",
            sql: || format!("SELECT ARRAY_CONSTRUCT(1{}) FROM a", " + 1".repeat(50_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a 70,000-cast :: chain inside a call is refused",
            sql: || format!("SELECT f(1{}) FROM a", "::INT".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a 70,000-subscript chain inside a call is refused",
            sql: || format!("SELECT f(v{}) FROM a", "[1]".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a 100,000-term + chain is flat, parses and serializes",
            sql: || format!("SELECT 1{}", "+1".repeat(100_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a || chain is flat and parses",
            sql: || format!("SELECT 'a'{}", "||'a'".repeat(70_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a :: chain is refused",
            sql: || format!("SELECT 1{}", "::INT".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "an = chain is refused",
            sql: || format!("SELECT 1{}", "=1".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a [1] chain is refused",
            sql: || format!("SELECT v{}", "[1]".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a .a chain is refused",
            sql: || format!("SELECT v{}", ".a".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a COLLATE chain is refused",
            sql: || format!("SELECT 'a'{}", " COLLATE 'en'".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "an IS NULL chain is refused",
            sql: || format!("SELECT 1{}", " IS NULL".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "an IN chain is refused",
            sql: || format!("SELECT 1{}", " IN (1)".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a : path chain is one path and parses",
            sql: || format!("SELECT v{}", ":a".repeat(70_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a chain alternating = and IS NULL is refused",
            sql: || format!("SELECT 1{}", "=1 IS NULL".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a :: chain cut off by a syntax error is refused",
            sql: || format!("SELECT 1{}::", "::INT".repeat(70_000)),
            expected_error: Some("Expected identifier"),
        },
    ];
    for test_case in test_cases {
        let sql = (test_case.sql)();
        let outcome = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || guarded_parse(&sql))
            .expect("the parse thread starts")
            .join()
            .expect("the parse thread finishes");
        assert_eq!(
            outcome
                .as_ref()
                .err()
                .map(|error| error.contains(test_case.expected_error.unwrap_or("\0"))),
            test_case.expected_error.map(|_| true),
            "{}: {outcome:?}",
            test_case.description
        );
    }
}

#[test]
fn given_a_deeply_nested_type_when_parsing_on_a_2_mb_thread_then_512_levels_parse_and_more_are_refused(
) {
    const NESTING: Option<&str> = Some("E_GUARD_NESTING_DEPTH_EXCEEDED");
    const PARSER_DEPTH: Option<&str> = Some("E_GUARD_PARSER_DEPTH_EXCEEDED");
    const FUNCTION_NESTING: Option<&str> = Some("E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED");
    let test_cases = [
        TypeDepthTestCase {
            description: "[] suffixes in a Snowflake CAST",
            dialect: crate::Dialect::Snowflake,
            sql: |n| format!("SELECT CAST(NULL AS INT{})", "[]".repeat(n)),
            expected_errors: [None, NESTING, NESTING, NESTING],
        },
        TypeDepthTestCase {
            description: "[3] suffixes in a DuckDB CAST",
            dialect: crate::Dialect::DuckDB,
            sql: |n| format!("SELECT CAST(NULL AS INT{})", "[3]".repeat(n)),
            expected_errors: [None, NESTING, NESTING, NESTING],
        },
        TypeDepthTestCase {
            description: "[] suffixes in a DuckDB :: cast",
            dialect: crate::Dialect::DuckDB,
            sql: |n| format!("SELECT x::INT{} FROM t", "[]".repeat(n)),
            expected_errors: [None, NESTING, NESTING, NESTING],
        },
        TypeDepthTestCase {
            description: "[] suffixes on a Snowflake column type",
            dialect: crate::Dialect::Snowflake,
            sql: |n| format!("CREATE TABLE t (c INT{})", "[]".repeat(n)),
            expected_errors: [None, NESTING, NESTING, NESTING],
        },
        TypeDepthTestCase {
            description: "nested ARRAY<> in a Snowflake CAST",
            dialect: crate::Dialect::Snowflake,
            sql: |n| {
                format!(
                    "SELECT CAST(NULL AS {}INT{})",
                    "ARRAY<".repeat(n),
                    ">".repeat(n)
                )
            },
            expected_errors: [None, NESTING, PARSER_DEPTH, PARSER_DEPTH],
        },
        TypeDepthTestCase {
            description: "ARRAY<> levels alternating with short [] runs",
            dialect: crate::Dialect::Snowflake,
            sql: |n| {
                let levels = n / 4;
                let inner = "[][][]>".repeat(levels);
                let outer = "[]".repeat(n - 4 * levels);
                format!(
                    "SELECT CAST(NULL AS {}INT{inner}{outer})",
                    "ARRAY<".repeat(levels)
                )
            },
            expected_errors: [None, NESTING, PARSER_DEPTH, PARSER_DEPTH],
        },
        TypeDepthTestCase {
            description: "ARRAY( levels each wrapping 60 [] suffixes",
            dialect: crate::Dialect::Snowflake,
            sql: |n| constructor_levels(n, "ARRAY("),
            expected_errors: [None, NESTING, FUNCTION_NESTING, FUNCTION_NESTING],
        },
        TypeDepthTestCase {
            description: "OBJECT( levels each wrapping 60 [] suffixes",
            dialect: crate::Dialect::Snowflake,
            sql: |n| constructor_levels(n, "OBJECT(a "),
            expected_errors: [None, NESTING, FUNCTION_NESTING, FUNCTION_NESTING],
        },
        TypeDepthTestCase {
            description: "MAP( values each wrapping 60 [] suffixes",
            dialect: crate::Dialect::Snowflake,
            sql: |n| constructor_levels(n, "MAP(VARCHAR, "),
            expected_errors: [None, NESTING, FUNCTION_NESTING, FUNCTION_NESTING],
        },
        TypeDepthTestCase {
            description: "STRUCT( levels each wrapping 60 [] suffixes",
            dialect: crate::Dialect::DuckDB,
            sql: |n| constructor_levels(n, "STRUCT(a "),
            expected_errors: [None, NESTING, FUNCTION_NESTING, FUNCTION_NESTING],
        },
    ];
    for test_case in test_cases {
        for (index, levels) in TYPE_DEPTHS.into_iter().enumerate() {
            let sql = (test_case.sql)(levels);
            let dialect = test_case.dialect;
            let outcome = std::thread::Builder::new()
                .stack_size(2 << 20)
                .spawn(move || guarded_parse_in(&sql, dialect))
                .expect("the parse thread starts")
                .join()
                .expect("the parse thread finishes");
            let code = outcome.as_ref().err().and_then(|error| {
                error
                    .split(|c: char| !(c.is_ascii_uppercase() || c == '_'))
                    .find(|word| word.starts_with("E_"))
            });
            assert_eq!(
                code, test_case.expected_errors[index],
                "{} at {levels} levels: {outcome:?}",
                test_case.description
            );
        }
    }
}

#[test]
fn given_long_chains_and_deep_nesting_on_a_2_mb_thread_when_cloning_comparing_formatting_serializing_or_dropping_then_none_overflows(
) {
    let test_cases = [
        TreeOperationTestCase {
            description: "clone",
            operation: |tree| tree.clone().len() == tree.len(),
            expected_completed: true,
        },
        TreeOperationTestCase {
            description: "equality with a clone, and inequality between different statements",
            operation: |tree| {
                let clone = tree.clone();
                clone == tree && clone[1..] != tree[..tree.len() - 1]
            },
            expected_completed: true,
        },
        TreeOperationTestCase {
            description: "Debug formatting",
            operation: |tree| debug_length(&tree) > 0,
            expected_completed: true,
        },
        TreeOperationTestCase {
            description: "serialization",
            operation: |tree| serde_json::to_writer(std::io::sink(), &tree).is_ok(),
            expected_completed: true,
        },
        TreeOperationTestCase {
            description: "drop",
            operation: |tree| {
                drop(tree);
                true
            },
            expected_completed: true,
        },
    ];
    for test_case in test_cases {
        let operation = test_case.operation;
        let trees = std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn(deep_trees)
            .expect("the parsing thread starts")
            .join()
            .expect("the trees parse");
        let completed = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || operation(trees))
            .expect("the thread starts")
            .join()
            .expect("the thread finishes");
        assert_eq!(
            completed, test_case.expected_completed,
            "{}",
            test_case.description
        );
    }
}
