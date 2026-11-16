"""Reading tree dumps, cases, the checker's family results and the reviews files."""

from __future__ import annotations

import json
import tomllib
from pathlib import Path
from typing import Any

from scripts.rebound_reviews.constants import APPROVED, FAILED_VERDICT, REBOUND_NOTE
from scripts.rebound_reviews.models import CaseLine, Rebound


def dump(*, path: Path) -> dict[str, str]:
    """`<case_id>\\t<compact tree JSON or REJECTED>` lines, as a tree dumper prints, by case."""
    trees: dict[str, str] = {}
    for line in path.read_text().splitlines():
        if line:
            case_id, tree = line.split("\t", 1)
            trees[case_id] = tree
    return trees


def cases(*, directory: Path) -> dict[str, dict[str, Any]]:
    """Every recorded case in the `*.jsonl` files of `directory`, by id."""
    rows: dict[str, dict[str, Any]] = {}
    for path in sorted(directory.glob("*.jsonl")):
        for line in path.read_text().splitlines():
            if line:
                row: dict[str, Any] = json.loads(line)
                rows[row["case_id"]] = row
    return rows


def form_cases(
    *, paths: list[Path], recorded: dict[str, dict[str, Any]]
) -> dict[str, list[CaseLine]]:
    """Each form's citing cases from `syntax-coverage family <f> --json` output, once each."""
    forms: dict[str, list[CaseLine]] = {}
    for path in paths:
        for form in json.loads(path.read_text()):
            seen: dict[str, CaseLine] = {}
            for obligation in form["obligations"]:
                for case in obligation["cases"]:
                    local: dict[str, Any] = case["local"]
                    row: dict[str, Any] = recorded.get(case["case_id"], {})
                    seen[case["case_id"]] = CaseLine(
                        case_id=case["case_id"],
                        sql_sha256=case["sql_sha256"],
                        verdict=local["verdict"]
                        if local["verdict"] != FAILED_VERDICT
                        else f"failed:{local['failure']}",
                        tree_sha256=local.get("tree_sha256"),
                        expect=row.get("expect"),
                        stage=row.get("stage"),
                        phase=case.get("phase"),
                    )
            forms[form["id"]] = [seen[case_id] for case_id in sorted(seen)]
    return forms


def rebound(*, reviews: Path) -> list[Rebound]:
    """Each form a re-bound review names, with the fingerprint the review before it approved."""
    found: list[Rebound] = []
    for path in sorted(reviews.glob("*.toml")):
        approved: dict[str, str] = {}
        for review in tomllib.loads(path.read_text()).get("review", []):
            if review.get("verdict") != APPROVED:
                continue
            fingerprints: dict[str, str] = review.get("fingerprints", {})
            if str(review.get("note", "")).startswith(REBOUND_NOTE):
                found += [
                    Rebound(
                        review_id=review["id"],
                        form_id=form_id,
                        fingerprint=fingerprints.get(form_id, ""),
                        previous=approved.get(form_id),
                    )
                    for form_id in review.get("forms", [])
                ]
            approved.update(fingerprints)
    return found
