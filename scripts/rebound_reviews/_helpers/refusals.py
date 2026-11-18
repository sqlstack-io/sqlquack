"""Why one re-bound form is refused: its trees or cases changed by more than its class allows."""

from __future__ import annotations

import json
from dataclasses import replace
from typing import Any

from scripts.rebound_reviews._helpers.fingerprints import form_fingerprint, tree_sha256
from scripts.rebound_reviews._helpers.trees import quantifier_difference
from scripts.rebound_reviews.constants import (
    ACCEPTED_VERDICT,
    EXPECT_ACCEPT,
    EXPECT_KEY,
    REJECTED,
    STAGE_KEY,
    STAGE_OF_PHASE,
)
from scripts.rebound_reviews.models import CaseLine, Rebound


def refusal(
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


def stage_refusal(
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
        if now.get(STAGE_KEY) != then.get(STAGE_KEY):
            stage_reason: str | None = new_stage_refusal(line=line, case=now)
            if stage_reason is not None:
                return f"{line.case_id}: {stage_reason}"
        reviewed.append(replace(line, stage=then.get(STAGE_KEY)))
    if form_fingerprint(lines=reviewed, trees=current) != claim.previous:
        return "the old cases are not the ones the earlier review approved"
    return None


def new_stage_refusal(*, line: CaseLine, case: dict[str, Any]) -> str | None:
    """Why a case's changed stage is refused: unless accepted, its receipt phase's stage."""
    stage: Any = case.get(STAGE_KEY)
    if case.get(EXPECT_KEY) != EXPECT_ACCEPT:
        return "its stage changed, but it is not expected to be accepted"
    if line.phase not in STAGE_OF_PHASE:
        return f"its stage changed, but {line.phase!r} is no receipt phase that gives a stage"
    if stage != STAGE_OF_PHASE[line.phase]:
        return f"stage {stage!r} is not the stage of its receipt's {line.phase}"
    return None
