"""Hazard cases: every declared rule form, with each hazard of each argument family it accepts."""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any

from scripts.case_generators.models import Case, CaseFile
from scripts.hazard_cases._helpers.cases import form_cases
from scripts.hazard_cases.constants import (
    COVERAGE_DOMAINS,
    COVERAGE_HAZARDS,
    HAZARDS,
    POLICY_FILE,
    REPOSITORY,
)


def hazard_files(repository: Path = REPOSITORY) -> list[CaseFile]:
    """One file per declared rule and category, each case left out when an earlier file holds it."""
    catalogue: dict[str, Any] = tomllib.loads((repository / COVERAGE_HAZARDS).read_text())
    policy: dict[str, Any] = tomllib.loads((repository / POLICY_FILE).read_text())
    required: dict[str, tuple[str, ...]] = {
        name: tuple(table.get("required", ())) for name, table in policy["edge_cases"].items()
    }
    seen: set[tuple[object, ...]] = set()
    files: list[CaseFile] = []
    for path in sorted((repository / COVERAGE_DOMAINS).glob("*.toml")):
        for rule in tomllib.loads(path.read_text()).get("rule", []):
            grouped: dict[str, tuple[str, list[Case]]] = _rule_cases(
                rule=rule, families=catalogue["families"], required=required
            )
            for category, (construct, cases) in sorted(grouped.items()):
                kept: list[Case] = []
                for case in cases:
                    key: tuple[object, ...] = (case.columns, case.rows, case.expression, case.query)
                    if key not in seen:
                        seen.add(key)
                        kept.append(case)
                if kept:
                    files.append(
                        CaseFile(
                            generator=HAZARDS,
                            name=f"{rule['id'].removeprefix('rule.')}__{category}",
                            category=category,
                            construct=construct,
                            cases=tuple(kept),
                            header=(f"Hazards of the forms rule {rule['id']} declares.",),
                        )
                    )
    return files


def _rule_cases(
    *, rule: dict[str, Any], families: dict[str, Any], required: dict[str, tuple[str, ...]]
) -> dict[str, tuple[str, list[Case]]]:
    """A rule's cases by category, forms then probes, each with the construct of its first form."""
    grouped: dict[str, tuple[str, list[Case]]] = {}
    labelled: list[tuple[str, dict[str, Any]]] = [
        (f"_{index}" if index else "", form) for index, form in enumerate(rule.get("form", []))
    ]
    labelled.extend((f"_probe_{index}", probe) for index, probe in enumerate(rule.get("probe", [])))
    for suffix, form in labelled:
        if form.get("template") is None or form.get("category") is None:
            continue
        category: str = form["category"]
        construct, cases = grouped.setdefault(category, (form["construct"], []))
        cases.extend(
            form_cases(form=form, suffix=suffix, families=families, tags=required[category])
        )
    return grouped
