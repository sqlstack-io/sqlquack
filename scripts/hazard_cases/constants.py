"""Where the coverage files live, and how generated hazard cases are named and built."""

from __future__ import annotations

from pathlib import Path

REPOSITORY: Path = Path(__file__).resolve().parents[2]
COVERAGE_DOMAINS: str = "coverage/domains"
COVERAGE_HAZARDS: str = "coverage/hazards.toml"
POLICY_FILE: str = "crates/sqlquack-policy/policy.toml"
HAZARDS: str = "hazards@1"
HAZARD_ARGUMENT_PREFIX: str = "x"
HAZARD_GROUP_COLUMN: str = "g"
COMBINED_HAZARDS: str = "hazards"
EXPRESSION_SLOT: str = "{expression}"
HAZARD_QUERY: str = "SELECT g, {expression} AS r FROM input GROUP BY g ORDER BY g"
LITERAL_KIND: str = "literal"
VALUE_KIND: str = "value"
NULL_FAMILY: str = "null"
PARTITION_FACET: str = "partition"
PARTITION_CLAUSE: str = "PARTITION BY g"
SECOND_GROUP_COLUMN: str = "g2"
SEVERAL_PARTITIONS: str = "several"
UNPARTITIONED: str = "none"

TIED_DIVERGENCE: str = "tied"
