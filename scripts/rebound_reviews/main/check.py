"""Refuse each re-bound review whose forms changed by more than its class allows.

A set_quantifier re-bind allows only the call flag read as its quantifier in the trees. A stage
re-bind allows only the cases' stage, and only where it is the stage of the case's receipt phase.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import replace
from pathlib import Path
from typing import Any

from scripts.rebound_reviews._helpers.fingerprints import form_fingerprint, tree_sha256
from scripts.rebound_reviews._helpers.inputs import cases, dump, form_cases, rebound
from scripts.rebound_reviews._helpers.trees import quantifier_difference
from scripts.rebound_reviews.constants import (
    ACCEPTED_VERDICT,
    REJECTED,
    STAGE_CLASS,
    STAGE_KEY,
    STAGE_OF_PHASE,
)
from scripts.rebound_reviews.models import CaseLine, Rebound


def run_rebound_check(argv: list[str] | None = None) -> int:
    """Fail unless each re-bound form's reviewed trees, by fingerprint, map to its current ones."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reviews", type=Path, required=True, help="the coverage root's reviews/")
    parser.add_argument("--cases", type=Path, required=True, help="the recorded cases/ directory")
    parser.add_argument(
        "--family",
        type=Path,
        action="append",
        required=True,
        help="`syntax-coverage family <f> --json` output",
    )
    parser.add_argument("--old", type=Path, help="trees from the parser the reviews read")
    parser.add_argument("--new", type=Path, help="trees from the current parser")
    parser.add_argument(
        "--old-cases", type=Path, help="the cases/ directory the stage re-binds' reviews read"
    )
    arguments: argparse.Namespace = parser.parse_args(argv)
    recorded: dict[str, dict[str, Any]] = cases(directory=arguments.cases)
    forms: dict[str, list[CaseLine]] = form_cases(paths=arguments.family, recorded=recorded)
    old: dict[str, str] = dump(path=arguments.old) if arguments.old else {}
    new: dict[str, str] = dump(path=arguments.new) if arguments.new else {}
    before: dict[str, dict[str, Any]] | None = (
        cases(directory=arguments.old_cases) if arguments.old_cases else None
    )
    claims: list[Rebound] = rebound(reviews=arguments.reviews)
    refused: list[str] = []
    for claim in claims:
        reason: str | None = (
            _stage_refusal(claim=claim, forms=forms, recorded=recorded, before=before)
            if claim.diff_class == STAGE_CLASS
            else _refusal(claim=claim, forms=forms, recorded=recorded, old=old, new=new)
        )
        if reason is not None:
            refused.append(f"{claim.review_id} {claim.form_id}: {reason}")
    for line in refused:
        print(f"rebound reviews: refused {line}", file=sys.stderr)
    reviews: int = len({claim.review_id for claim in claims})
    passed: int = len(claims) - len(refused)
    print(
        f"rebound reviews: {passed} of {len(claims)} forms in {reviews} reviews diff-checked; "
        f"{'FAILED' if refused or not claims else 'passed'}",
        file=sys.stderr,
    )
    return 1 if refused or not claims else 0


def _refusal(
    *,
    claim: Rebound,
    forms: dict[str, list[CaseLine]],
    recorded: dict[str, dict[str, Any]],
    old: dict[str, str],
    new: dict[str, str],
) -> str | None:
    """Why the re-bind of one form is refused; None when every check holds."""
    lines: list[CaseLine] | None = forms.get(claim.form_id)
    if lines is None:
        return "not in the checker's family results"
    if claim.previous is None:
        return "no earlier approved review records a fingerprint for it"
    current: dict[str, str | None] = {line.case_id: line.tree_sha256 for line in lines}
    if form_fingerprint(lines=lines, trees=current) != claim.fingerprint:
        return "the review's fingerprint is not the form's current one"
    reviewed: dict[str, str | None] = {}
    for line in lines:
        if line.case_id not in old or line.case_id not in new:
            return f"{line.case_id} is missing from a tree dump"
        accepted: bool = line.verdict == ACCEPTED_VERDICT
        if (old[line.case_id] != REJECTED) != accepted or (
            new[line.case_id] != REJECTED
        ) != accepted:
            return f"{line.case_id}: a dump's verdict is not the checker's {line.verdict}"
        if not accepted:
            continue
        if tree_sha256(new[line.case_id]) != line.tree_sha256:
            return f"{line.case_id}: the new dump is not the current parser's tree"
        reviewed[line.case_id] = tree_sha256(old[line.case_id])
        difference: str | None = quantifier_difference(
            old=json.loads(old[line.case_id]),
            new=json.loads(new[line.case_id]),
            sql=str(recorded.get(line.case_id, {}).get("sql", "")),
        )
        if difference is not None:
            return f"{line.case_id}: {difference}"
    if form_fingerprint(lines=lines, trees=reviewed) != claim.previous:
        return "the old trees are not the ones the earlier review approved"
    return None


def _stage_refusal(
    *,
    claim: Rebound,
    forms: dict[str, list[CaseLine]],
    recorded: dict[str, dict[str, Any]],
    before: dict[str, dict[str, Any]] | None,
) -> str | None:
    """Why a stage re-bind is refused; None when only stages changed, each to its receipt's."""
    lines: list[CaseLine] | None = forms.get(claim.form_id)
    if lines is None:
        return "not in the checker's family results"
    if claim.previous is None:
        return "no earlier approved review records a fingerprint for it"
    if before is None:
        return "no --old-cases to compare its cases with"
    current: dict[str, str | None] = {line.case_id: line.tree_sha256 for line in lines}
    if form_fingerprint(lines=lines, trees=current) != claim.fingerprint:
        return "the review's fingerprint is not the form's current one"
    reviewed: list[CaseLine] = []
    for line in lines:
        now: dict[str, Any] = recorded.get(line.case_id, {})
        then: dict[str, Any] | None = before.get(line.case_id)
        if then is None:
            return f"{line.case_id} is not in the old cases"
        changed: list[str] = sorted(
            key for key in (set(now) | set(then)) - {STAGE_KEY} if now.get(key) != then.get(key)
        )
        if changed:
            return f"{line.case_id}: {', '.join(changed)} changed, not only its stage"
        stage: Any = now.get(STAGE_KEY)
        if stage != then.get(STAGE_KEY) and stage != STAGE_OF_PHASE.get(str(line.phase)):
            return f"{line.case_id}: stage {stage!r} is not the stage of its receipt's {line.phase}"
        reviewed.append(replace(line, stage=then.get(STAGE_KEY)))
    if form_fingerprint(lines=reviewed, trees=current) != claim.previous:
        return "the old cases are not the ones the earlier review approved"
    return None
