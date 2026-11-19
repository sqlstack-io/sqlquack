//! Deep and chained statements parsed on a small stack: refused past the limit, never aborted.

use crate::parse::main::parse::parse;
use crate::parse::tests::test_types::{ChainTestCase, NestingTestCase};

#[test]
fn given_deep_nesting_when_parsing_on_a_small_stack_then_it_is_refused_not_aborted() {
    let too_deep: Option<&str> = Some("the statement nests too deeply");
    let test_cases = [
        NestingTestCase {
            description: "1,500 parentheses around a table in FROM",
            sql: || format!("SELECT * FROM {}t{}", "(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "3,000 parenthesised joins in FROM",
            sql: || {
                let open: String = "(".repeat(3000);
                format!("SELECT * FROM {open}t{}", " JOIN u ON TRUE)".repeat(3000))
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "3,000 STREAM reads of one table",
            sql: || format!("SELECT * FROM {}t", "STREAM ".repeat(3000)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 subqueries nested in FROM",
            sql: || {
                let open: String = "(SELECT * FROM ".repeat(1500);
                format!("SELECT * FROM {open}t{}", ")".repeat(1500))
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 scalar subqueries",
            sql: || format!("SELECT {}1{}", "(SELECT ".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 EXISTS subqueries",
            sql: || {
                let open: String = "EXISTS (SELECT * FROM t WHERE ".repeat(1500);
                format!("SELECT * FROM t WHERE {open}TRUE{}", ")".repeat(1500))
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 IN subqueries",
            sql: || {
                format!(
                    "SELECT 1 IN {}(1){}",
                    "(SELECT 1 IN ".repeat(1500),
                    ")".repeat(1500)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 parentheses around a query",
            sql: || format!("{}SELECT 1{}", "(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 parentheses around a set operation operand",
            sql: || {
                format!(
                    "{}SELECT 1{} UNION SELECT 2",
                    "(".repeat(1500),
                    ")".repeat(1500)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 parentheses around an expression",
            sql: || format!("SELECT {}1{}", "(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 nested calls",
            sql: || format!("SELECT {}1{}", "abs(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "6,000 nested subscripts",
            sql: || format!("SELECT {}0{}", "a[".repeat(6000), "]".repeat(6000)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 nested ARRAY types",
            sql: || {
                format!(
                    "SELECT CAST(NULL AS {}INT{})",
                    "ARRAY<".repeat(1500),
                    ">".repeat(1500)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 nested STRUCT types",
            sql: || {
                format!(
                    "SELECT CAST(NULL AS {}INT{})",
                    "STRUCT<a: ".repeat(1500),
                    ">".repeat(1500)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "past the limit, every other reading fails at once instead of retrying",
            sql: || {
                format!(
                    "SELECT 1 IN {}(1){}",
                    "(SELECT 1 IN ".repeat(400),
                    ")".repeat(400)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "40 parentheses around a table in FROM parse",
            sql: || format!("SELECT * FROM {}t{}", "(".repeat(40), ")".repeat(40)),
            expected_refusal: None,
        },
        NestingTestCase {
            description: "40 nested subqueries in FROM parse",
            sql: || {
                let open: String = "(SELECT * FROM ".repeat(40);
                format!("SELECT * FROM {open}t{}", ")".repeat(40))
            },
            expected_refusal: None,
        },
        NestingTestCase {
            description: "CACHE SELECT keeps text too deep for its documented form as a gap",
            sql: || {
                format!(
                    "CACHE SELECT {}1{} FROM t1",
                    "(".repeat(700),
                    ")".repeat(700)
                )
            },
            expected_refusal: None,
        },
        NestingTestCase {
            description: "40 nested subscripts parse",
            sql: || format!("SELECT {}0{}", "a[".repeat(40), "]".repeat(40)),
            expected_refusal: None,
        },
    ];
    for test_case in test_cases {
        let sql: String = (test_case.sql)();
        let refusal: Option<String> = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || parse(&sql).err().map(|error| error.message))
            .expect("the parse thread starts")
            .join()
            .expect("the parse thread finishes");
        assert_eq!(
            refusal.as_deref(),
            test_case.expected_refusal,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_nested_chains_when_parsing_on_a_small_stack_then_trees_past_the_limit_are_refused() {
    let too_deep: Option<&str> = Some("the statement nests too deeply");
    let test_cases = [
        ChainTestCase {
            description: "10 parenthesised levels of 60 links",
            levels: 10,
            length: 60,
            expected_refusal: None,
        },
        ChainTestCase {
            description: "60 parenthesised levels of 60 links",
            levels: 60,
            length: 60,
            expected_refusal: None,
        },
        ChainTestCase {
            description: "130 parenthesised levels of 60 links, just inside the tree limit",
            levels: 130,
            length: 60,
            expected_refusal: None,
        },
        ChainTestCase {
            description: "60 parenthesised levels of 200 links",
            levels: 60,
            length: 200,
            expected_refusal: too_deep,
        },
        ChainTestCase {
            description: "150 parenthesised levels of 60 links",
            levels: 150,
            length: 60,
            expected_refusal: too_deep,
        },
    ];
    let links: [(&str, &str, &str); 7] = [
        ("SELECT ", "x", "::INT"),
        ("SELECT ", "x", " + 1"),
        ("SELECT ", "x", " || 'a'"),
        ("SELECT ", "x", " AND TRUE"),
        ("SELECT ", "x", " OR TRUE"),
        ("SELECT ", "x", "[1]"),
        ("", "SELECT 1", " UNION ALL SELECT 1"),
    ];
    for test_case in test_cases {
        for (prefix, base, link) in links {
            let level: String = format!("{})", link.repeat(test_case.length));
            let sql: String = format!(
                "{prefix}{}{base}{}",
                "(".repeat(test_case.levels),
                level.repeat(test_case.levels)
            );
            let refusal: Option<String> = std::thread::Builder::new()
                .stack_size(2 << 20)
                .spawn(move || parse(&sql).err().map(|error| error.message))
                .expect("the parse thread starts")
                .join()
                .expect("the parse thread finishes");
            assert_eq!(
                refusal.as_deref(),
                test_case.expected_refusal,
                "{} of `{link}`",
                test_case.description
            );
        }
    }
}
