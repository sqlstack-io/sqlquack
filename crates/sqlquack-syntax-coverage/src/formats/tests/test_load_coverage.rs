use std::collections::BTreeSet;
use std::path::Path;

use crate::formats::constants::{COVERAGE_ROOT, FAMILIES};
use crate::formats::main::load_coverage::load_coverage;
use crate::formats::tests::helpers::{receipt_line, replace, restated_page, valid_tree, write};
use crate::formats::tests::test_types::{
    CommittedTestCase, FormatFaultTestCase, LoadTestCase, LoadedCounts,
};

#[test]
fn given_the_committed_tree_when_loading_then_every_family_and_record_loads() {
    let test_cases = [CommittedTestCase {
        description: "the committed catalogue, obligations, cases and imported receipts",
        expected_families: &FAMILIES,
    }];
    for test_case in test_cases {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(COVERAGE_ROOT);
        let coverage = load_coverage(&root)
            .unwrap_or_else(|problems| panic!("{}: {problems:#?}", test_case.description));
        let families =
            |keys: Vec<&String>| -> BTreeSet<String> { keys.into_iter().cloned().collect() };
        let expected: BTreeSet<String> = test_case
            .expected_families
            .iter()
            .map(|family| (*family).to_owned())
            .collect();
        assert_eq!(
            [
                families(coverage.catalogue.keys().collect()),
                families(coverage.obligations.keys().collect()),
                families(coverage.cases.keys().collect()),
            ],
            [expected.clone(), expected.clone(), expected],
            "{}: {:?}",
            test_case.description,
            test_case.expected_families
        );
    }
}

#[test]
fn given_a_valid_tree_when_loading_then_every_record_is_kept() {
    let counts = |receipts: usize| LoadedCounts {
        sources: 2,
        indexes: 1,
        aliases: 2,
        forms: 2,
        definitions: 1,
        obligations: 4,
        cases: 1,
        receipts,
        reviews: 2,
        file_hashes: 2 * FAMILIES.len() + 2,
    };
    let test_cases = [
        LoadTestCase {
            description: "a tree with records of every kind, a prose form and a run report",
            tree: valid_tree,
            expected_counts: counts(1),
        },
        LoadTestCase {
            description: "a canonical-run receipt keeps only the query id hash and a Z time",
            tree: |root| {
                valid_tree(root);
                let hashed = receipt_line()
                    .replace(
                        "\"query_id\":\"01b2-0000\"",
                        "\"query_id\":null,\"query_id_sha256\":\
                         \"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\"",
                    )
                    .replace(".048112+00:00", "Z");
                write(root, "receipts/canonical.jsonl", &format!("{hashed}\n"));
            },
            expected_counts: counts(2),
        },
        LoadTestCase {
            description: "a DDL receipt from a run-owned database counts its completed statements",
            tree: |root| {
                valid_tree(root);
                let ddl = receipt_line()
                    .replace(
                        "\"capture_method\":\"execute\"",
                        "\"capture_method\":\"execute_ddl_in_run_database\"",
                    )
                    .replace(
                        "\"phase\":\"accepted\"",
                        "\"phase\":\"accepted\",\"statements_in_request\":3,\
                         \"completed_statements\":2",
                    );
                write(root, "receipts/ddl.jsonl", &format!("{ddl}\n"));
            },
            expected_counts: counts(2),
        },
        LoadTestCase {
            description: "a restated page on a form's also_sources, and a page review without a tree",
            tree: |root| {
                valid_tree(root);
                restated_page(root);
                replace(
                    root,
                    "reviews/exclusions.toml",
                    "parser_tree = \"0123456789abcdef0123456789abcdef01234567\"\n",
                    "",
                );
            },
            expected_counts: LoadedCounts {
                sources: 3,
                ..counts(1)
            },
        },
        LoadTestCase {
            description: "a folded form, absent from the source, keeps its now restated page",
            tree: |root| {
                valid_tree(root);
                restated_page(root);
                replace(
                    root,
                    "catalogue/query.toml",
                    "source = \"doc.select\"\nanchor = \"DEFINE VIEW\"",
                    "source = \"doc.select-copy\"\nanchor = \"DEFINE VIEW\"",
                );
            },
            expected_counts: LoadedCounts {
                sources: 3,
                ..counts(1)
            },
        },
        LoadTestCase {
            description: "an all-options interaction without target, pair or depths",
            tree: |root| {
                valid_tree(root);
                replace(
                    root,
                    "obligations/query.toml",
                    "kind = \"positive\"",
                    "kind = \"interaction_all_options\"",
                );
            },
            expected_counts: counts(1),
        },
        LoadTestCase {
            description: "a semantic refusal with its reason category and basis",
            tree: |root| {
                valid_tree(root);
                let refused = receipt_line().replace(
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"semantic_error\",\"reason_category\":\"missing_object\",\
                     \"reason_category_basis\":\"errno 2003\"",
                );
                write(root, "receipts/refused.jsonl", &format!("{refused}\n"));
            },
            expected_counts: counts(2),
        },
        LoadTestCase {
            description: "a tree without renames or reviews yet",
            tree: |root| {
                valid_tree(root);
                std::fs::remove_file(root.join("aliases.toml")).expect("remove");
                std::fs::remove_dir_all(root.join("reviews")).expect("remove");
            },
            expected_counts: LoadedCounts {
                aliases: 0,
                reviews: 0,
                file_hashes: 2 * FAMILIES.len() + 1,
                ..counts(1)
            },
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        (test_case.tree)(directory.path());
        let coverage = load_coverage(directory.path())
            .unwrap_or_else(|problems| panic!("{}: {problems:#?}", test_case.description));
        let counts = LoadedCounts {
            sources: coverage.sources.len(),
            indexes: coverage.indexes.len(),
            aliases: coverage.aliases.len(),
            forms: coverage.catalogue.values().map(Vec::len).sum(),
            definitions: coverage.definitions.values().map(Vec::len).sum(),
            obligations: coverage.obligations.values().map(Vec::len).sum(),
            cases: coverage.cases.values().map(Vec::len).sum(),
            receipts: coverage.receipts.len(),
            reviews: coverage.reviews.len(),
            file_hashes: coverage.file_hashes.len(),
        };
        assert_eq!(
            counts, test_case.expected_counts,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_format_fault_when_loading_then_the_problem_is_named() {
    let test_cases = [
        FormatFaultTestCase {
            description: "a deleted family catalogue",
            mutate: |root| std::fs::remove_file(root.join("catalogue/dml.toml")).expect("remove"),
            expected_problem: "catalogue/dml.toml: missing",
        },
        FormatFaultTestCase {
            description: "a deleted family case file",
            mutate: |root| {
                std::fs::remove_file(root.join("cases/scripting.jsonl")).expect("remove")
            },
            expected_problem: "cases/scripting.jsonl: missing",
        },
        FormatFaultTestCase {
            description: "a file naming no family",
            mutate: |root| write(root, "catalogue/misc.toml", ""),
            expected_problem: "catalogue/misc.toml: names no family",
        },
        FormatFaultTestCase {
            description: "a stray file among the reviews",
            mutate: |root| write(root, "reviews/notes.md", "notes"),
            expected_problem: "not a .toml file",
        },
        FormatFaultTestCase {
            description: "a missing sources file",
            mutate: |root| std::fs::remove_file(root.join("sources.toml")).expect("remove"),
            expected_problem: "sources.toml:",
        },
        FormatFaultTestCase {
            description: "a missing indexes file",
            mutate: |root| std::fs::remove_file(root.join("indexes.toml")).expect("remove"),
            expected_problem: "indexes.toml:",
        },
        FormatFaultTestCase {
            description: "an in-scope page without a family",
            mutate: |root| replace(root, "sources.toml", "family = \"query\"", "family = \"\""),
            expected_problem: "source doc.select: an out-of-scope page has no family",
        },
        FormatFaultTestCase {
            description: "an out-of-scope page with a family",
            mutate: |root| replace(root, "sources.toml", "family = \"\"", "family = \"query\""),
            expected_problem: "source doc.billing: an out-of-scope page has no family",
        },
        FormatFaultTestCase {
            description: "a source id outside doc.",
            mutate: |root| {
                replace(
                    root,
                    "sources.toml",
                    "id = \"doc.billing\"",
                    "id = \"page.billing\"",
                )
            },
            expected_problem: "source page.billing: not doc.<page-slug>",
        },
        FormatFaultTestCase {
            description: "an unknown source status",
            mutate: |root| {
                replace(
                    root,
                    "sources.toml",
                    "status = \"forms\"",
                    "status = \"partial\"",
                )
            },
            expected_problem: "unknown variant `partial`",
        },
        FormatFaultTestCase {
            description: "an unknown form field",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "anchor = \"Syntax\"",
                    "anchor = \"Syntax\"\nweight = 2",
                )
            },
            expected_problem: "unknown field `weight`",
        },
        FormatFaultTestCase {
            description: "a form id of another family",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "id = \"sf.query.select\"",
                    "id = \"sf.dml.select\"",
                )
            },
            expected_problem: "form sf.dml.select: not sf.query.<page-slug>[.<variant>] of family query",
        },
        FormatFaultTestCase {
            description: "a form whose family field disagrees with its file",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "family = \"query\"",
                    "family = \"dml\"",
                )
            },
            expected_problem: "form sf.query.select: not sf.query.<page-slug>[.<variant>] of family query",
        },
        FormatFaultTestCase {
            description: "a variant that reads as an obligation number",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "sf.query.select.define-view",
                    "sf.query.select.o001",
                )
            },
            expected_problem: "form sf.query.select.o001: not sf.query",
        },
        FormatFaultTestCase {
            description: "a form citing no pinned source",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "source = \"doc.select\"\nanchor = \"Syntax\"",
                    "source = \"doc.insert\"\nanchor = \"Syntax\"",
                )
            },
            expected_problem: "form sf.query.select: unknown source doc.insert",
        },
        FormatFaultTestCase {
            description: "a grammar edited without its hash",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "SELECT TOP <n>",
                    "SELECT TOP (<n>)",
                )
            },
            expected_problem: "form sf.query.select: grammar_sha256 does not match",
        },
        FormatFaultTestCase {
            description: "productions reordered without their hash",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "[\"SELECT [ DISTINCT | ALL ] <expr> [ , ... ]\", \"SELECT TOP <n> <expr>\"]",
                    "[\"SELECT TOP <n> <expr>\", \"SELECT [ DISTINCT | ALL ] <expr> [ , ... ]\"]",
                )
            },
            expected_problem: "form sf.query.select: grammar_sha256 does not match",
        },
        FormatFaultTestCase {
            description: "a prose form with a grammar",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "grammar = []",
                    "grammar = [\"DEFINE VIEW <name>\"]",
                )
            },
            expected_problem: "form sf.query.select.define-view: grammar_sha256 does not match",
        },
        FormatFaultTestCase {
            description: "a statement form with an empty grammar",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "kind = \"prose\"",
                    "kind = \"statement\"",
                )
            },
            expected_problem: "form sf.query.select.define-view: grammar_sha256 does not match",
        },
        FormatFaultTestCase {
            description: "a form using a definition its page lacks",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "uses = [\"expr\"]",
                    "uses = [\"expr\", \"query\"]",
                )
            },
            expected_problem: "form sf.query.select: uses query, which its page does not define",
        },
        FormatFaultTestCase {
            description: "a definition id that is not <source>::<name>",
            mutate: |root| {
                replace(
                    root,
                    "catalogue/query.toml",
                    "doc.select::expr",
                    "doc.select:expr",
                )
            },
            expected_problem: "definition doc.select:expr: not <source>::<name>",
        },
        FormatFaultTestCase {
            description: "obligation number zero",
            mutate: |root| replace(root, "obligations/query.toml", "o001", "o000"),
            expected_problem: "obligation sf.query.select.o000: not sf.query.<form>.o<NNN>",
        },
        FormatFaultTestCase {
            description: "an interaction with one member",
            mutate: |root| replace(root, "obligations/query.toml", "\"DISTINCT\", ", ""),
            expected_problem: "interaction needs a pair of two",
        },
        FormatFaultTestCase {
            description: "a family review excluding a page",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "form = \"sf.query.select.define-view\"",
                    "source = \"doc.billing\"",
                )
            },
            expected_problem: "review query-1: an exclusion needs exactly one target",
        },
        FormatFaultTestCase {
            description: "an interaction without the anchor of its usage note",
            mutate: |root| {
                replace(
                    root,
                    "obligations/query.toml",
                    "anchor = \"Usage notes\"\n",
                    "",
                )
            },
            expected_problem: "interaction needs the anchor of the usage note",
        },
        FormatFaultTestCase {
            description: "a reason category on an accepted receipt",
            mutate: |root| {
                replace(
                    root,
                    "receipts/first.jsonl",
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"accepted\",\"reason_category\":\"type\",\
                     \"reason_category_basis\":\"errno 1004\"",
                )
            },
            expected_problem: "receipt query.select-17: a reason category needs its basis",
        },
        FormatFaultTestCase {
            description: "a reason category without its basis",
            mutate: |root| {
                replace(
                    root,
                    "receipts/first.jsonl",
                    "\"phase\":\"accepted\"",
                    "\"phase\":\"runtime_error\",\"reason_category\":\"other\"",
                )
            },
            expected_problem: "receipt query.select-17: a reason category needs its basis",
        },
        FormatFaultTestCase {
            description: "an all-options interaction with a target",
            mutate: |root| {
                replace(
                    root,
                    "obligations/query.toml",
                    "kind = \"boundary_above\"",
                    "kind = \"interaction_all_options\"",
                )
            },
            expected_problem: "interaction_all_options takes no target",
        },
        FormatFaultTestCase {
            description: "depths on a kind that takes none",
            mutate: |root| {
                replace(
                    root,
                    "obligations/query.toml",
                    "kind = \"nesting_depth\"",
                    "kind = \"positive\"",
                )
            },
            expected_problem: "no other kind takes either",
        },
        FormatFaultTestCase {
            description: "a boundary obligation without a target",
            mutate: |root| replace(root, "obligations/query.toml", "target = \"TOP <n>\"\n", ""),
            expected_problem: "BoundaryAbove needs a target",
        },
        FormatFaultTestCase {
            description: "an unknown obligation kind",
            mutate: |root| {
                replace(
                    root,
                    "obligations/query.toml",
                    "kind = \"positive\"",
                    "kind = \"weighted\"",
                )
            },
            expected_problem: "unknown variant `weighted`",
        },
        FormatFaultTestCase {
            description: "an obligation id declared twice",
            mutate: |root| replace(root, "obligations/query.toml", "o002", "o001"),
            expected_problem: "sf.query.select.o001: declared twice",
        },
        FormatFaultTestCase {
            description: "case SQL edited without its hash",
            mutate: |root| {
                replace(
                    root,
                    "cases/query.jsonl",
                    "\"sql\":\"SELECT 17\"",
                    "\"sql\":\"SELECT 31\"",
                )
            },
            expected_problem: "case query.select-17: a hash does not match",
        },
        FormatFaultTestCase {
            description: "case setup edited without its hash",
            mutate: |root| replace(root, "cases/query.jsonl", "VALUES (17)", "VALUES (31)"),
            expected_problem: "case query.select-17: a hash does not match",
        },
        FormatFaultTestCase {
            description: "case session edited without its hash",
            mutate: |root| replace(root, "cases/query.jsonl", "false", "true"),
            expected_problem: "case query.select-17: a hash does not match",
        },
        FormatFaultTestCase {
            description: "a case without obligations",
            mutate: |root| replace(root, "cases/query.jsonl", "\"sf.query.select.o001\"", ""),
            expected_problem: "case query.select-17: needs obligation ids of family query",
        },
        FormatFaultTestCase {
            description: "a case citing an obligation of another family",
            mutate: |root| {
                replace(
                    root,
                    "cases/query.jsonl",
                    "sf.query.select.o001",
                    "sf.dml.insert.o001",
                )
            },
            expected_problem: "case query.select-17: needs obligation ids of family query",
        },
        FormatFaultTestCase {
            description: "a case id holding whitespace",
            mutate: |root| {
                replace(
                    root,
                    "cases/query.jsonl",
                    "query.select-17",
                    "query select-17",
                )
            },
            expected_problem: "case query select-17: id is empty, too long or holds whitespace",
        },
        FormatFaultTestCase {
            description: "one case id in two family files",
            mutate: |root| {
                let line = std::fs::read_to_string(root.join("cases/query.jsonl")).expect("read");
                write(
                    root,
                    "cases/dml.jsonl",
                    &line.replace("sf.query.select.o001", "sf.dml.insert.o001"),
                );
            },
            expected_problem: "query.select-17: declared twice",
        },
        FormatFaultTestCase {
            description: "a blank line between cases",
            mutate: |root| {
                let line = std::fs::read_to_string(root.join("cases/query.jsonl")).expect("read");
                write(root, "cases/query.jsonl", &format!("\n{line}"));
            },
            expected_problem: "cases/query.jsonl:1:",
        },
        FormatFaultTestCase {
            description: "a receipt holding an address",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &receipt_line().replace("run-1", "someone@example.com"),
                )
            },
            expected_problem: "receipt query.select-17: holds an account, host, URL or address",
        },
        FormatFaultTestCase {
            description: "a receipt of another format",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &receipt_line().replace("receipt/1", "receipt/2"),
                )
            },
            expected_problem: "format sqlquack-syntax-receipt/2 is not sqlquack-syntax-receipt/1",
        },
        FormatFaultTestCase {
            description: "a receipt with an unknown phase",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &receipt_line()
                        .replace("\"phase\":\"accepted\"", "\"phase\":\"probably_fine\""),
                )
            },
            expected_problem: "unknown variant `probably_fine`",
        },
        FormatFaultTestCase {
            description: "a receipt with an unknown field",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &receipt_line().replace("\"outcome\"", "\"verdict\":\"ok\",\"outcome\""),
                )
            },
            expected_problem: "unknown field `verdict`",
        },
        FormatFaultTestCase {
            description: "a receipt with an upper-case hash",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &receipt_line().replace("aaaa", "AAAA"),
                )
            },
            expected_problem: "receipt query.select-17: a hash is not a lower-case SHA-256",
        },
        FormatFaultTestCase {
            description: "a receipt whose session parameters changed after hashing",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &receipt_line().replace(":30}", ":31}"),
                )
            },
            expected_problem: "receipt query.select-17: session_sha256 does not match its parameters",
        },
        FormatFaultTestCase {
            description: "a receipt with a local time",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &receipt_line().replace("+00:00", "+01:00"),
                )
            },
            expected_problem: "recorded_at is not RFC 3339 in UTC",
        },
        FormatFaultTestCase {
            description: "a receipt with an empty fraction of a second",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &receipt_line().replace(".048112", "."),
                )
            },
            expected_problem: "recorded_at is not RFC 3339 in UTC",
        },
        FormatFaultTestCase {
            description: "a receipt time with a multi-byte character where seconds end",
            mutate: |root| {
                write(
                    root,
                    "receipts/first.jsonl",
                    &receipt_line().replace("22:45:26.048112", "22:45:2é.048112"),
                )
            },
            expected_problem: "recorded_at is not RFC 3339 in UTC",
        },
        FormatFaultTestCase {
            description: "a stray file beside the receipts",
            mutate: |root| write(root, "receipts/notes.json", "{}"),
            expected_problem: "receipts/notes.json: not a .jsonl file",
        },
        FormatFaultTestCase {
            description: "a review of an abbreviated tree",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "0123456789abcdef0123456789abcdef01234567",
                    "0123456",
                )
            },
            expected_problem: "review query-1: parser_tree is not a Git tree id",
        },
        FormatFaultTestCase {
            description: "a review accepting an exclusion of another family",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "sf.query.select.define-view",
                    "sf.dml.insert",
                )
            },
            expected_problem: "an exclusion needs exactly one target: a form id of its family",
        },
        FormatFaultTestCase {
            description: "a review accepting an exclusion without a reason",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "valid only inside project files",
                    " ",
                )
            },
            expected_problem: "review query-1: an exclusion needs a reason",
        },
        FormatFaultTestCase {
            description: "an alias renaming a form to itself",
            mutate: |root| replace(root, "aliases.toml", "sf.query.selects", "sf.query.select"),
            expected_problem: "alias sf.query.select: needs distinct form ids",
        },
        FormatFaultTestCase {
            description: "a split naming one id twice",
            mutate: |root| {
                replace(
                    root,
                    "aliases.toml",
                    "\"sf.functions.dayofweek\"",
                    "\"sf.functions.dayofmonth\"",
                )
            },
            expected_problem: "alias sf.functions.day: needs distinct form ids",
        },
        FormatFaultTestCase {
            description: "an alias into nothing",
            mutate: |root| replace(root, "aliases.toml", "[\"sf.query.select\"]", "[]"),
            expected_problem: "alias sf.query.selects: needs distinct form ids",
        },
        FormatFaultTestCase {
            description: "an exclusion naming both a form and a source",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "form = \"sf.query.select.define-view\"",
                    "form = \"sf.query.select.define-view\"\nsource = \"doc.select\"",
                )
            },
            expected_problem: "review query-1: an exclusion needs exactly one",
        },
        FormatFaultTestCase {
            description: "a page review excluding a form",
            mutate: |root| {
                replace(
                    root,
                    "reviews/exclusions.toml",
                    "source = \"doc.billing\"",
                    "form = \"sf.query.select\"",
                )
            },
            expected_problem: "review pages-1: an exclusion needs exactly one",
        },
        FormatFaultTestCase {
            description: "a page review binding catalogue hashes",
            mutate: |root| {
                replace(
                    root,
                    "reviews/exclusions.toml",
                    "sources_sha256",
                    "catalogue_sha256",
                )
            },
            expected_problem: "review pages-1: a family review binds its catalogue",
        },
        FormatFaultTestCase {
            description: "a family review binding the sources file",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "obligations_sha256",
                    "sources_sha256",
                )
            },
            expected_problem: "review query-1: a family review binds its catalogue",
        },
        FormatFaultTestCase {
            description: "a family review without a parser tree",
            mutate: |root| {
                replace(
                    root,
                    "reviews/query.toml",
                    "parser_tree = \"0123456789abcdef0123456789abcdef01234567\"\n",
                    "",
                )
            },
            expected_problem: "review query-1: parser_tree is not a Git tree id; only a page review",
        },
        FormatFaultTestCase {
            description: "a restated page that no form lists",
            mutate: |root| {
                restated_page(root);
                replace(
                    root,
                    "catalogue/query.toml",
                    "also_sources = [\"doc.select-copy\"]\n",
                    "",
                );
            },
            expected_problem: "source doc.select-copy: restated, but no form lists it",
        },
        FormatFaultTestCase {
            description: "a form whose own page is restated",
            mutate: |root| {
                restated_page(root);
                replace(
                    root,
                    "catalogue/query.toml",
                    "source = \"doc.select\"\nalso_sources = [\"doc.select-copy\"]",
                    "source = \"doc.select-copy\"\nalso_sources = [\"doc.select\"]",
                );
            },
            expected_problem: "form sf.query.select: its source doc.select-copy is restated",
        },
        FormatFaultTestCase {
            description: "a form listing its own page as a restatement",
            mutate: |root| {
                restated_page(root);
                replace(
                    root,
                    "catalogue/query.toml",
                    "[\"doc.select-copy\"]",
                    "[\"doc.select\", \"doc.select-copy\"]",
                );
            },
            expected_problem: "form sf.query.select: also_sources must be sorted, distinct",
        },
        FormatFaultTestCase {
            description: "a form restated on an unpinned page",
            mutate: |root| {
                restated_page(root);
                replace(
                    root,
                    "catalogue/query.toml",
                    "[\"doc.select-copy\"]",
                    "[\"doc.nowhere\", \"doc.select-copy\"]",
                );
            },
            expected_problem: "form sf.query.select: unknown also_source doc.nowhere",
        },
        FormatFaultTestCase {
            description: "a form restated on an out-of-scope page",
            mutate: |root| {
                restated_page(root);
                replace(
                    root,
                    "catalogue/query.toml",
                    "[\"doc.select-copy\"]",
                    "[\"doc.billing\", \"doc.select-copy\"]",
                );
            },
            expected_problem: "form sf.query.select: also_source doc.billing has no in-scope syntax",
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        valid_tree(directory.path());
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
