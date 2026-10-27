use crate::policy::_helpers::parsing::parse_policy;
use crate::policy::main::matching_type_equality::matching_type_equality;
use crate::policy::models::Policy;

pub(crate) const VALID_POLICY: &str = r#"
schema_version = 1

[comparator]
column_names = "case_insensitive"
null_order = "nulls_last_on_asc_first_on_desc"
timestamp_tz = "instant"

[comparator.float_ulps]
float32 = 0
float64 = 0

[[comparator.type_equality]]
snowflake = "NUMBER(p,s)"
duckdb = "DECIMAL(p,s)"
reason = "same values"

[[comparator.type_equality]]
snowflake = "VARCHAR(n)"
duckdb = "VARCHAR"
reason = "length does not change values"

[session_profiles.utc-v1]
description = "UTC"
duckdb_time_zone = "UTC"

[session_profiles.utc-v1.snowflake]
TIMEZONE = "UTC"
WEEK_START = 0

[edge_cases.numeric]
description = "numbers"
required = ["null", "zero"]

[conversion_budget]
milliseconds = 120000
size_ratio = 2000
minimum_bytes = 4194304

[divergences.hash]
constructs = ["fn:HASH"]
default = "warn"
strict = "refuse"
reason = "values differ, consistently"
"#;

pub(crate) fn observed_error(replace: &str, with: &str, fragment: Option<&str>) -> Option<String> {
    let message = parse_policy(&VALID_POLICY.replacen(replace, with, 1))
        .err()
        .map(|error| error.to_string())?;
    Some(
        fragment
            .filter(|fragment| message.contains(fragment))
            .map_or(message.clone(), str::to_owned),
    )
}

pub(crate) fn valid_policy() -> Policy {
    parse_policy(VALID_POLICY).expect("the base test policy is valid")
}

pub(crate) fn equality_reason(snowflake: &str, duckdb: &str) -> Option<String> {
    let policy = valid_policy();
    matching_type_equality(&policy.comparator.type_equality, snowflake, duckdb)
        .map(|equality| equality.reason.clone())
}
