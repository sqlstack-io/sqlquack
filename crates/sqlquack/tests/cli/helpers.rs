use std::path::{Path, PathBuf};
use std::process::Command;

use walkdir::WalkDir;

use crate::test_types::CommandOutcome;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_files(from: &Path, to: &Path) {
    for entry in WalkDir::new(from)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
    {
        let target = to.join(entry.path().strip_prefix(from).expect("inside the tree"));
        std::fs::create_dir_all(target.parent().expect("a parent directory"))
            .expect("directory created");
        std::fs::copy(entry.path(), &target).expect("file copied");
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
    copy_files(&root.join("cases"), &directory.path().join("cases"));
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
