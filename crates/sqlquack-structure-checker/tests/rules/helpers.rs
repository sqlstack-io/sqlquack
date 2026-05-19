use crate::test_types;
use sqlquack_structure_checker::rules;
use std::fs;
use std::io::Write;
use std::path;

pub(crate) fn make_temp_repo(test_case: &test_types::CheckRepoTestCase) -> path::PathBuf {
    let repo_root = std::env::temp_dir().join(format!(
        "sqlquack-structure-rules-test-{}-{}",
        std::process::id(),
        unique_suffix()
    ));

    write_repo_files(&repo_root, compliant_repo_files().as_slice());
    write_repo_files(&repo_root, test_case.repo_files);

    repo_root
}

pub(crate) fn collect_violation_codes(repo_root: &path::Path) -> Vec<&'static str> {
    rules::check_repo(repo_root)
        .iter()
        .map(|violation| violation.code)
        .collect()
}

pub(crate) fn collect_test_violation_codes(repo_root: &path::Path) -> Vec<&'static str> {
    rules::check_repo(repo_root)
        .iter()
        .filter(|violation| violation.code.starts_with("SQT"))
        .map(|violation| violation.code)
        .collect()
}

fn compliant_repo_files() -> Vec<test_types::RepoFile> {
    vec![
        test_types::RepoFile {
            path: "Cargo.toml",
            contents: "[workspace]\nmembers = [\"crates/sqlquack\", \"crates/example\"]\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/Cargo.toml",
            contents: "[package]\nname = \"sqlquack\"\nversion = \"0.1.0\"\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/lib.rs",
            contents: "mod analysis;\nmod ast;\npub use analysis::models::AnalysisResult;\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/mod.rs",
            contents: "pub(crate) mod models;\nmod helpers;\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/models.rs",
            contents: "pub struct AnalysisResult { pub diagnostics_count: usize }\nimpl AnalysisResult { pub(crate) fn is_empty(&self) -> bool { self.diagnostics_count == 0 } }\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/types.rs",
            contents: "pub type AnalysisId = u32;\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/constants.rs",
            contents: "pub(crate) const MAX_SCOPE_DEPTH: usize = 1024;\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/main/mod.rs",
            contents: "pub mod analyze_query;\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/main/analyze_query.rs",
            contents: "use crate::analysis::models;\npub(crate) fn analyze_query() -> models::AnalysisResult { models::AnalysisResult { diagnostics_count: 0 } }\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/helpers/mod.rs",
            contents: "pub(super) mod scope_builder;\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/analysis/helpers/scope_builder.rs",
            contents: "use crate::analysis::models;\nstruct ScopeFrame { depth: usize }\npub(super) fn build_scope() -> models::AnalysisResult { let _frame = ScopeFrame { depth: 0 }; models::AnalysisResult { diagnostics_count: 0 } }\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/ast/mod.rs",
            contents: "pub(crate) mod models;\n",
        },
        test_types::RepoFile {
            path: "crates/sqlquack/src/ast/models.rs",
            contents: "pub struct Query;\n",
        },
        test_types::RepoFile {
            path: "crates/example/Cargo.toml",
            contents: "[package]\nname = \"example\"\nversion = \"0.1.0\"\n",
        },
        test_types::RepoFile {
            path: "crates/example/tests/rules.rs",
            contents: "#[path = \"rules/helpers.rs\"]\nmod helpers;\n#[path = \"rules/role_files.rs\"]\nmod role_files;\n#[path = \"rules/test_types.rs\"]\nmod test_types;\n",
        },
        test_types::RepoFile {
            path: "crates/example/tests/rules/role_files.rs",
            contents: "use crate::helpers;\nuse crate::test_types;\nconst TEST_CASES: &[test_types::CheckRepoTestCase] = &[];\n#[test]\nfn given_input_when_running_then_reports_output() {\n    for test_case in TEST_CASES {\n        assert_eq!(1, test_case.expected_count, \"{}\", test_case.description);\n    }\n}\n",
        },
        test_types::RepoFile {
            path: "crates/example/tests/rules/helpers.rs",
            contents: "use crate::test_types;\npub(crate) fn build_count(test_case: &test_types::CheckRepoTestCase) -> usize { test_case.expected_count }\n",
        },
        test_types::RepoFile {
            path: "crates/example/tests/rules/test_types.rs",
            contents: "pub(crate) struct CheckRepoTestCase {\n    pub(crate) description: &'static str,\n    pub(crate) expected_count: usize,\n}\n",
        },
    ]
}

fn write_repo_files(repo_root: &path::Path, files: &[test_types::RepoFile]) {
    for repo_file in files {
        let file_path = repo_root.join(repo_file.path);
        fs::create_dir_all(
            file_path
                .parent()
                .expect("test repo file path should have a parent"),
        )
        .expect("test repo file parent should be created");
        let mut file = fs::File::create(&file_path).expect("test repo file should be created");
        file.write_all(repo_file.contents.as_bytes())
            .expect("test repo file should be written");
    }
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos()
}
