use std::collections::BTreeSet;

use crate::formats::_helpers::dialect::current_dialect;
use crate::formats::main::load_coverage::load_coverage;
use crate::formats::main::load_focus::load_focus;
use crate::formats::tests::helpers::{
    engine_receipt_line, postgres_tree, receipt_line, replace, valid_tree, write,
};
use crate::formats::tests::test_types::{
    DialectLoadTestCase, FocusFaultTestCase, FocusTestCase, FormatFaultTestCase,
};

const POSTGRES_FAMILIES: [&str; 8] = [
    "ddl",
    "dml",
    "expressions",
    "functions",
    "query",
    "security_admin",
    "transaction_session",
    "utility",
];

#[test]
fn given_a_dialect_file_when_loading_then_the_root_loads_as_that_dialect() {
    let test_cases = [
        DialectLoadTestCase {
            description: "a Postgres root: eight families, pg ids and an engine receipt",
            tree: postgres_tree,
            expected_dialect: "postgres",
            expected_families: &POSTGRES_FAMILIES,
            expected_first_query_form: "pg.query.select",
            expected_engine_versions: &[Some("18.6")],
        },
        DialectLoadTestCase {
            description: "a Postgres root whose /2 receipt has no query id or warehouse",
            tree: |root| {
                postgres_tree(root);
                let second = engine_receipt_line().replace("\"run-1\"", "\"run-2\"");
                write(root, "receipts/second.jsonl", &format!("{second}\n"));
            },
            expected_dialect: "postgres",
            expected_families: &POSTGRES_FAMILIES,
            expected_first_query_form: "pg.query.select",
            expected_engine_versions: &[Some("18.6"), Some("18.6")],
        },
        DialectLoadTestCase {
            description: "a Postgres receipt whose message names @ operators",
            tree: |root| {
                postgres_tree(root);
                replace(
                    root,
                    "receipts/first.jsonl",
                    "\"run-1\"",
                    "\"operator does not exist: @-@ box; a @> b; a <@ b; @@ q\"",
                );
            },
            expected_dialect: "postgres",
            expected_families: &POSTGRES_FAMILIES,
            expected_first_query_form: "pg.query.select",
            expected_engine_versions: &[Some("18.6")],
        },
        DialectLoadTestCase {
            description: "a Postgres receipt whose parse-only pass was a SQL function body",
            tree: |root| {
                postgres_tree(root);
                replace(
                    root,
                    "receipts/first.jsonl",
                    "\"parser_check\":\"do_block\"",
                    "\"parser_check\":\"sql_function\"",
                );
            },
            expected_dialect: "postgres",
            expected_families: &POSTGRES_FAMILIES,
            expected_first_query_form: "pg.query.select",
            expected_engine_versions: &[Some("18.6")],
        },
        DialectLoadTestCase {
            description: "a Postgres receipt judged on the whole text of a multi-statement case",
            tree: |root| {
                postgres_tree(root);
                replace(
                    root,
                    "receipts/first.jsonl",
                    "\"parser_check\":\"do_block\"",
                    "\"parser_check\":\"whole_text\"",
                );
            },
            expected_dialect: "postgres",
            expected_families: &POSTGRES_FAMILIES,
            expected_first_query_form: "pg.query.select",
            expected_engine_versions: &[Some("18.6")],
        },
        DialectLoadTestCase {
            description: "a Postgres case and receipt captured in autocommit in a throwaway database",
            tree: |root| {
                postgres_tree(root);
                for (path, field) in [
                    ("cases/query.jsonl", "capture"),
                    ("receipts/first.jsonl", "capture_method"),
                ] {
                    replace(
                        root,
                        path,
                        &format!("\"{field}\":\"execute_in_rolled_back_transaction\""),
                        &format!("\"{field}\":\"execute_autocommit_in_throwaway_database\""),
                    );
                }
            },
            expected_dialect: "postgres",
            expected_families: &POSTGRES_FAMILIES,
            expected_first_query_form: "pg.query.select",
            expected_engine_versions: &[Some("18.6")],
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        (test_case.tree)(directory.path());
        let coverage = load_coverage(directory.path())
            .unwrap_or_else(|problems| panic!("{}: {problems:#?}", test_case.description));
        let families: BTreeSet<&str> = coverage.catalogue.keys().map(String::as_str).collect();
        let versions: Vec<Option<&str>> = coverage
            .receipts
            .iter()
            .map(|receipt| receipt.engine_version.as_deref())
            .collect();
        assert_eq!(
            (
                current_dialect().name.as_str(),
                families,
                coverage.catalogue["query"][0].id.as_str(),
                versions,
            ),
            (
                test_case.expected_dialect,
                test_case.expected_families.iter().copied().collect(),
                test_case.expected_first_query_form,
                test_case.expected_engine_versions.to_vec(),
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_dialect_fault_when_loading_then_the_problem_is_named() {
    let test_cases = [
        FormatFaultTestCase {
            description: "a Postgres tree without its dialect file is held to Snowflake's families",
            mutate: |root| std::fs::remove_file(root.join("dialect.toml")).expect("remove"),
            expected_problem: "catalogue/scripting.toml: missing",
        },
        FormatFaultTestCase {
            description: "a Postgres tree without its dialect file is held to receipt format /1",
            mutate: |root| std::fs::remove_file(root.join("dialect.toml")).expect("remove"),
            expected_problem: "format sqlquack-syntax-receipt/2 is not sqlquack-syntax-receipt/1",
        },
        FormatFaultTestCase {
            description: "a Snowflake form id under the Postgres prefix",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "\"pg.query.select\"",
                    "\"sf.query.select\"",
                );
            },
            expected_problem: "sf.query.select",
        },
        FormatFaultTestCase {
            description: "a /1 receipt in a /2 root",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &format!("{}\n", receipt_line()),
                )
            },
            expected_problem: "format sqlquack-syntax-receipt/1 is not sqlquack-syntax-receipt/2",
        },
        FormatFaultTestCase {
            description: "a /2 receipt without its parser check",
            mutate: |root| {
                let line = engine_receipt_line().replace("\"parser_check\":\"do_block\",", "");
                write(root, "receipts/first.jsonl", &format!("{line}\n"));
            },
            expected_problem: "/2 carries engine",
        },
        FormatFaultTestCase {
            description: "a /2 receipt holding an email address",
            mutate: |root| {
                replace(
                    root,
                    "receipts/first.jsonl",
                    "\"run-1\"",
                    "\"user@example.com\"",
                )
            },
            expected_problem: "receipt query.select-17: holds an account, host, URL or address",
        },
        FormatFaultTestCase {
            description: "a /2 receipt holding a user at a host",
            mutate: |root| {
                replace(
                    root,
                    "receipts/first.jsonl",
                    "\"run-1\"",
                    "\"kevin@host.local\"",
                )
            },
            expected_problem: "receipt query.select-17: holds an account, host, URL or address",
        },
        FormatFaultTestCase {
            description: "a /2 receipt from the retired live-only parser check",
            mutate: |root| {
                let line = engine_receipt_line().replace(
                    "\"parser_check\":\"do_block\"",
                    "\"parser_check\":\"live_only\"",
                );
                write(root, "receipts/first.jsonl", &format!("{line}\n"));
            },
            expected_problem: "/2 carries engine",
        },
        FormatFaultTestCase {
            description: "a /2 receipt that still names a warehouse",
            mutate: |root| {
                let line = engine_receipt_line().replace(
                    "\"engine\":",
                    "\"warehouse_version\":\"10.40.100\",\"engine\":",
                );
                write(root, "receipts/first.jsonl", &format!("{line}\n"));
            },
            expected_problem: "/2 carries engine",
        },
        FormatFaultTestCase {
            description: "a /2 root case with a Snowflake capture",
            mutate: |root| {
                replace(
                    root,
                    "cases/query.jsonl",
                    "\"capture\":\"execute_in_rolled_back_transaction\"",
                    "\"capture\":\"execute\"",
                )
            },
            expected_problem: "case query.select-17: receipt format /2 roots take exactly",
        },
        FormatFaultTestCase {
            description: "a /2 receipt with a Snowflake capture",
            mutate: |root| {
                let line = engine_receipt_line().replace(
                    "\"capture_method\":\"execute_in_rolled_back_transaction\"",
                    "\"capture_method\":\"execute\"",
                );
                write(root, "receipts/first.jsonl", &format!("{line}\n"));
            },
            expected_problem: "receipt query.select-17: engine captures go with format /2",
        },
        FormatFaultTestCase {
            description: "a family the checker does not know",
            mutate: |root| replace(root, "dialect.toml", "\"utility\"", "\"plpgsql\""),
            expected_problem: "dialect.toml: families are distinct known families",
        },
        FormatFaultTestCase {
            description: "a /2 root without syntax codes",
            mutate: |root| replace(root, "dialect.toml", "syntax_codes = [\"42601\"]\n", ""),
            expected_problem: "dialect.toml: syntax_codes",
        },
        FormatFaultTestCase {
            description: "an unknown key",
            mutate: |root| {
                replace(
                    root,
                    "dialect.toml",
                    "local_parser",
                    "parser = 1\nlocal_parser",
                )
            },
            expected_problem: "unknown field `parser`",
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        postgres_tree(directory.path());
        (test_case.mutate)(directory.path());
        let problems = load_coverage(directory.path()).expect_err(test_case.description);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains(test_case.expected_problem)),
            "{}: {problems:#?}",
            test_case.description
        );
    }
}

#[test]
fn given_a_valid_focus_file_when_loading_then_its_entries_are_kept_in_order() {
    let test_cases = [
        FocusTestCase {
            description: "a catalogued Postgres form with sorted uses and a note",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"pg.query.select\"\nuses = [\"A\", \"B\", \"E\"]\nnote = \"models\"\n",
            expected_entries: 1,
        },
        FocusTestCase {
            description: "a Snowflake focus file on the Snowflake root",
            tree: valid_tree,
            focus: "dialect = \"snowflake\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"sf.query.select\"\nuses = [\"H\"]\n",
            expected_entries: 1,
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        (test_case.tree)(directory.path());
        let coverage = load_coverage(directory.path())
            .unwrap_or_else(|problems| panic!("{}: {problems:#?}", test_case.description));
        write(directory.path(), "focus.toml", test_case.focus);
        let focus = load_focus(&directory.path().join("focus.toml"), &coverage)
            .unwrap_or_else(|problems| panic!("{}: {problems:#?}", test_case.description));
        assert_eq!(
            focus.entries.len(),
            test_case.expected_entries,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_focus_fault_when_loading_then_it_fails_closed() {
    let test_cases = [
        FocusFaultTestCase {
            description: "an id the catalogue does not hold",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"pg.query.selectt\"\nuses = [\"A\"]\n",
            expected_problem: "pg.query.selectt is not a catalogued form",
        },
        FocusFaultTestCase {
            description: "a Snowflake id on the Postgres root",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"sf.query.select\"\nuses = [\"A\"]\n",
            expected_problem: "sf.query.select is not a catalogued form",
        },
        FocusFaultTestCase {
            description: "a form absent from its source",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"pg.query.select.define-view\"\nuses = [\"A\"]\n",
            expected_problem: "is absent from its source",
        },
        FocusFaultTestCase {
            description: "a form listed twice",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"pg.query.select\"\nuses = [\"A\"]\n\
                    [[focus]]\nform = \"pg.query.select\"\nuses = [\"B\"]\n",
            expected_problem: "pg.query.select is listed twice",
        },
        FocusFaultTestCase {
            description: "unsorted uses",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"pg.query.select\"\nuses = [\"B\", \"A\"]\n",
            expected_problem: "uses are not sorted distinct letters",
        },
        FocusFaultTestCase {
            description: "a use letter past H",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"pg.query.select\"\nuses = [\"A\", \"I\"]\n",
            expected_problem: "uses are not sorted distinct letters",
        },
        FocusFaultTestCase {
            description: "no uses",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"pg.query.select\"\nuses = []\n",
            expected_problem: "uses are not sorted distinct letters",
        },
        FocusFaultTestCase {
            description: "an unknown key",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"pg.query.select\"\nuses = [\"A\"]\npriority = 1\n",
            expected_problem: "unknown field `priority`",
        },
        FocusFaultTestCase {
            description: "a Postgres focus file on the Snowflake root",
            tree: valid_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n\
                    [[focus]]\nform = \"sf.query.select\"\nuses = [\"A\"]\n",
            expected_problem: "dialect postgres is not the root's snowflake",
        },
        FocusFaultTestCase {
            description: "an abbreviated SQLBuild commit",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a\"\n\
                    [[focus]]\nform = \"pg.query.select\"\nuses = [\"A\"]\n",
            expected_problem: "sqlbuild_commit is not a full",
        },
        FocusFaultTestCase {
            description: "no focus forms",
            tree: postgres_tree,
            focus: "dialect = \"postgres\"\nsqlbuild_commit = \"5f451f88a0000000000000000000000000000000\"\n",
            expected_problem: "lists no focus form",
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        (test_case.tree)(directory.path());
        let coverage = load_coverage(directory.path())
            .unwrap_or_else(|problems| panic!("{}: {problems:#?}", test_case.description));
        write(directory.path(), "focus.toml", test_case.focus);
        let problems = load_focus(&directory.path().join("focus.toml"), &coverage)
            .expect_err(test_case.description);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains(test_case.expected_problem)),
            "{}: {problems:#?}",
            test_case.description
        );
    }
}
