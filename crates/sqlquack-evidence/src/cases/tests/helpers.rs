use std::path::Path;

use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::Policy;

use crate::cases::main::load_cases::load_cases;
use crate::cases::models::LoadedCase;

pub(crate) const POLICY: &str = r#"
schema_version = 1

[comparator]
column_names = "case_insensitive"
null_order = "nulls_last_on_asc_first_on_desc"
timestamp_tz = "instant"
type_equality = []

[comparator.float_ulps]
float32 = 0
float64 = 0

[session_profiles.utc-v1]
description = "UTC"
duckdb_time_zone = "UTC"

[session_profiles.utc-v1.snowflake]
TIMEZONE = "UTC"

[session_profiles.utc-v2]
description = "UTC, weeks from Monday"
duckdb_time_zone = "UTC"

[session_profiles.utc-v2.snowflake]
TIMEZONE = "UTC"
WEEK_START = 1

[edge_cases.string]
description = "strings"
required = ["null", "empty"]
"#;

pub(crate) const CASE_FILE: &str = r#"
category = "string"
construct = "fn:CONCAT"
session_profile = "utc-v1"

[[case]]
id = "concat"
description = "CONCAT with NULL and empty inputs."
columns = [{ name = "a", type = "VARCHAR" }, { name = "b", type = "VARCHAR" }]
rows = [["'gift'", "NULL"], ["''", "'wrap'"]]
expression = "CONCAT(a, b)"
tags = ["null", "empty"]

[[case]]
id = "sorted"
description = "Sorted values."
columns = [{ name = "a", type = "VARCHAR" }]
rows = [["'b'"], ["'a'"]]
query = "SELECT a FROM input ORDER BY a"
order = "ordered"
tags = []
"#;

fn policy(directory: &Path) -> Policy {
    let path = directory.join("sqlquack.toml");
    std::fs::write(&path, POLICY).expect("policy written");
    load_policy(&path).expect("the test policy is valid")
}

pub(crate) fn load_variant(
    directory: &Path,
    replace: &str,
    with: &str,
) -> Result<Vec<LoadedCase>, String> {
    let cases = directory.join("cases");
    std::fs::create_dir_all(cases.join("string")).expect("cases directory");
    std::fs::write(
        cases.join("string").join("concat.toml"),
        CASE_FILE.replacen(replace, with, 1),
    )
    .expect("case file written");
    load_cases(&cases, &policy(directory)).map_err(|error| error.to_string())
}

pub(crate) fn hashes(replace: &str, with: &str) -> Vec<String> {
    let directory = tempfile::tempdir().expect("temporary directory");
    load_variant(directory.path(), replace, with)
        .expect("the variant loads")
        .into_iter()
        .map(|case| case.hash)
        .collect()
}

pub(crate) fn observed_problem(
    replace: &str,
    with: &str,
    fragment: Option<&str>,
) -> Option<String> {
    let directory = tempfile::tempdir().expect("temporary directory");
    let message = load_variant(directory.path(), replace, with).err()?;
    Some(
        fragment
            .filter(|fragment| message.contains(fragment))
            .map_or(message.clone(), str::to_owned),
    )
}

pub(crate) fn source_sql_of(reference: &str) -> Option<String> {
    let directory = tempfile::tempdir().expect("temporary directory");
    load_variant(directory.path(), "", "")
        .expect("the case file loads")
        .into_iter()
        .find(|case| case.reference == reference)
        .map(|case| case.source_sql)
}

/// How many single-row sources the case `reference` gets once `replace` becomes `with`.
pub(crate) fn row_source_count(reference: &str, (replace, with): (&str, &str)) -> Option<usize> {
    let directory = tempfile::tempdir().expect("temporary directory");
    load_variant(directory.path(), replace, with)
        .expect("the variant loads")
        .into_iter()
        .find(|case| case.reference == reference)
        .map(|case| case.row_sources.len())
}
