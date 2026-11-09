//! Fixtures for the translation measure: a fake `sqlquack` and by-design files.

use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::check::tests::helpers::{sha256, write};
use crate::formats::main::load_coverage::load_coverage;
use crate::translation::main::measure_translation::measure_translation;
use crate::translation::models::{TranslationInputs, TranslationReport};

const WAIT_REASON: &str = "DuckDB has no session to suspend";

/// A fake `sqlquack` that converts `SELECT 17` and runs `convert` for any other SQL.
pub(crate) fn fake_sqlquack(directory: &Path, convert: &str) -> std::path::PathBuf {
    let path = directory.join("sqlquack");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\nsql=$(cat)\nschema=\nwhile [ $# -gt 0 ]; do [ \"$1\" = --project ] && schema=$2/sources/census.yml; shift; done\n\
             if [ \"$sql\" = 'SELECT 17' ]; then echo '{{\"converted\":true,\"refusals\":[]}}'; exit 0; fi\n{convert}\n"
        ),
    )
    .expect("fake");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
    path
}

/// The fake's lines refusing the SQL for one construct.
pub(crate) fn refused(category: &str, construct: &str) -> String {
    format!(
        "echo '{{\"converted\":false,\"refusals\":[{{\"category\":\"{category}\",\"construct\":\"{construct}\"}}]}}'; exit 1"
    )
}

/// Writes no by-design files.
pub(crate) fn no_by_design(_: &Path) {}

/// Writes one by-design entry without a sign-off.
pub(crate) fn by_design_entry(directory: &Path) {
    write(
        directory,
        "by-design.toml",
        &format!(
            "[[by_design]]\nconstruct = \"fn:SYSTEM$WAIT\"\nreason = \"{WAIT_REASON}\"\nduckdb_note = \"none\"\n"
        ),
    );
}

/// Writes one by-design entry and a reviewer's sign-off of its reason.
pub(crate) fn signed_off(directory: &Path) {
    by_design_entry(directory);
    write(
        directory,
        "reviews/by-design.toml",
        &format!(
            "[[signoff]]\nconstruct = \"fn:SYSTEM$WAIT\"\nreason_sha256 = \"{}\"\nreviewed_utc = \"2026-11-04T10:00:00Z\"\n",
            sha256(WAIT_REASON)
        ),
    );
}

/// A sign-off of an earlier reason: the entry's reason has changed since the reviewer read it.
pub(crate) fn stale_signoff(directory: &Path) {
    by_design_entry(directory);
    write(
        directory,
        "reviews/by-design.toml",
        &format!(
            "[[signoff]]\nconstruct = \"fn:SYSTEM$WAIT\"\nreason_sha256 = \"{}\"\nreviewed_utc = \"2026-11-04T10:00:00Z\"\n",
            sha256("an earlier reason")
        ),
    );
}

/// The measure over the coverage tree at `root`, converting with `binary`.
pub(crate) fn measure(
    root: &Path,
    by_design: &BTreeSet<String>,
    binary: &Path,
) -> Result<TranslationReport, Vec<String>> {
    let coverage = load_coverage(root, None).expect("coverage loads");
    let work = tempfile::tempdir().expect("work");
    let policy = root.join("policy.toml");
    measure_translation(&TranslationInputs {
        coverage: &coverage,
        root,
        by_design,
        binary,
        policy: &policy,
        work: work.path(),
        family: None,
        jobs: 2,
    })
}
