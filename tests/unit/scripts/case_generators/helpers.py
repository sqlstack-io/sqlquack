"""Tampering with a generated directory."""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any

from scripts.case_generators.constants import GENERATED_DIRECTORY
from scripts.case_generators.models import Case
from scripts.hazard_cases.constants import COVERAGE_DOMAINS, COVERAGE_HAZARDS, REPOSITORY
from scripts.hazard_cases.main.hazard_files import hazard_files


def untouched(_: Path) -> None:
    """Leaves the generated files as written."""


def edited(repository: Path) -> None:
    """Hand-edits one generated file."""
    path: Path = repository / GENERATED_DIRECTORY / "time_zones" / "convert_timezone.toml"
    path.write_text(path.read_text().replace("'UTC'", "'Etc/UTC'", 1))


def stray(repository: Path) -> None:
    """Adds a file no generator writes."""
    (repository / GENERATED_DIRECTORY / "time_zones" / "extra.toml").write_text("")


def removed(repository: Path) -> None:
    """Deletes one generated file."""
    (repository / GENERATED_DIRECTORY / "time_zones" / "convert_timezone.toml").unlink()


HAZARD_DOMAIN: str = """version = 1

[[rule]]
id = "rule.key_lookup"

[[rule.form]]
construct = "fn:GET"
arguments = [{ families = ["variant"] }, { families = ["text"], kinds = ["value"] }]
template = "GET({0}, {1})"
category = "semi-structured"
divergences = { "1.text.slash_prefix" = "json_fidelity" }

[[rule.probe]]
construct = "fn:GET"
arguments = [{ families = ["variant"] }, { families = ["text"] }]
template = "GET({0}, {1})"
category = "semi-structured"
extra_rows = [["PARSE_JSON('{\\"k\\":1}')", "'k'"]]

[[rule]]
id = "rule.listagg"

[[rule.form]]
construct = "node:within_group"
arguments = [{ families = ["text"] }, { families = ["integer"], grammar = true }]
template = "LISTAGG({0}) WITHIN GROUP (ORDER BY {1})"
query = "SELECT g, {expression} AS r FROM input GROUP BY g ORDER BY g"
category = "aggregate"
ordinal = ["1"]
row_groups = true
divergences = { tied = "listagg_order" }

[[rule.form]]
construct = "node:within_group"
arguments = [{ families = ["text"] }, { families = ["text"], grammar = true }]
template = "LISTAGG({0}, ',') WITHIN GROUP (ORDER BY {1})"
query = "SELECT g, {expression} AS r FROM input GROUP BY g ORDER BY g"
category = "aggregate"
ordinal = ["1"]
divergences = { tied = "listagg_order" }

[[rule]]
id = "rule.text_pair"

[[rule.form]]
construct = "fn:CONTAINS"
arguments = [{ families = ["text"], kinds = ["value"] }, { families = ["text"], kinds = ["value"] }]
template = "CONTAINS({0}, {1})"
category = "semi-structured"

[[rule]]
id = "rule.boolean_triple"

[[rule.form]]
construct = "fn:DECODE"
arguments = [{ families = ["boolean"] }, { families = ["boolean"] }, { families = ["boolean"] }]
template = "DECODE({0}, {1}, {2})"
category = "semi-structured"
"""

HAZARD_CATALOGUE: str = """version = 1

[families.text]
sql_type = "VARCHAR"
benign = "'a'"
other = "'b'"
third = "'ab'"

[[families.text.hazard]]
id = "empty"
description = "Empty text."
cells = ["''"]

[[families.text.hazard]]
id = "dollar_prefix"
description = "A path-like key."
cells = ["'$.a'"]

[[families.text.hazard]]
id = "slash_prefix"
description = "A pointer-like key."
cells = ["'/a'"]

[families.variant]
sql_type = "VARIANT"
benign = "PARSE_JSON('{}')"
other = "PARSE_JSON('[]')"

[[families.variant.hazard]]
id = "duplicate_keys"
description = "A repeated key."
cells = ["PARSE_JSON('{\\"a\\":1,\\"a\\":2}')"]
isolate = true

[[families.variant.hazard]]
id = "nesting"
description = "Nesting."
cells = ["PARSE_JSON('{\\"a\\":{\\"b\\":1}}')"]

[[families.variant.hazard]]
id = "long_number"
description = "A number no converted input holds."
cells = ["PARSE_JSON('{\\"a\\":1.0}')"]
divergence = "json_fidelity"

[[families.variant.hazard]]
id = "unrecorded"
description = "A value no recording can hold."
cells = ["PARSE_JSON('[2]')"]
unrecordable = "The connector cannot convert it."

[families.integer]
sql_type = "NUMBER(38,0)"
benign = "1"
other = "2"

[families.boolean]
sql_type = "BOOLEAN"
benign = "TRUE"
other = "FALSE"
"""

HAZARD_POLICY: str = """[edge_cases.semi-structured]
required = ["null"]

[edge_cases.aggregate]
required = ["null"]
"""


def hazard_cases(repository: Path) -> dict[str, Case]:
    """The hazard cases generated for one GET form in a repository made under `repository`."""
    (repository / "coverage/domains").mkdir(parents=True)
    (repository / "coverage/domains/get.toml").write_text(HAZARD_DOMAIN)
    (repository / "coverage/hazards.toml").write_text(HAZARD_CATALOGUE)
    (repository / "sqlquack.toml").write_text(HAZARD_POLICY)
    cases: dict[str, Case] = {}
    for case_file in hazard_files(repository):
        for case in case_file.cases:
            cases[case.id] = case
    return cases


def coverage_entries() -> dict[str, set[str]]:
    """Each catalogue family's hazard ids, as `families.<name>`, and every declared facet."""
    facets: set[str] = set()
    for path in (REPOSITORY / COVERAGE_DOMAINS).glob("*.toml"):
        for rule in tomllib.loads(path.read_text()).get("rule", []):
            for form in rule.get("form", []):
                facets.update(form.get("facets", {}))
    entries: dict[str, set[str]] = {"facets": facets}
    catalogue: dict[str, Any] = tomllib.loads((REPOSITORY / COVERAGE_HAZARDS).read_text())
    for name, family in catalogue["families"].items():
        entries[f"families.{name}"] = {hazard["id"] for hazard in family.get("hazard", [])}
    return entries
