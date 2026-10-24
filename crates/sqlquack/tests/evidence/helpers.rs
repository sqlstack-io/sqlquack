use std::path::{Path, PathBuf};
use std::process::Command;

use crate::test_types::CommandOutcome;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("directory created");
    for entry in std::fs::read_dir(from).expect("directory listed").flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("file copied");
        }
    }
}

/// Runs a command in a copy of the repository's policy and cases, with no recorded results.
pub(crate) fn run_on_repository_cases(arguments: &[&str], mention: &str) -> CommandOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = repository_root();
    std::fs::copy(
        root.join("sqlquack.toml"),
        directory.path().join("sqlquack.toml"),
    )
    .expect("policy copied");
    copy_tree(&root.join("cases"), &directory.path().join("cases"));
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(arguments)
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    let pending = directory.path().join("evidence/pending");
    CommandOutcome {
        exit_code: output.status.code(),
        pending_files: std::fs::read_dir(pending).map_or(0, Iterator::count),
        stderr_mentions: String::from_utf8_lossy(&output.stderr).contains(mention),
    }
}
