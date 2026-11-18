"""A form fingerprint as `syntax-coverage` computes it, so a review's can be recomputed."""

from __future__ import annotations

import hashlib
import json

from scripts.rebound_reviews.constants import ACCEPTED_VERDICT, REJECTED_VERDICT
from scripts.rebound_reviews.models import CaseLine


def form_fingerprint(*, lines: list[CaseLine], trees: dict[str, str | None]) -> str:
    """SHA-256 over one line per case, by id, with each accepted case's tree hash from `trees`."""
    text: str = "".join(
        f"{line.case_id}\t{line.sql_sha256}\t{_verdict(line=line, tree=trees.get(line.case_id))}\t"
        f"{json.dumps(line.expect)}\t{json.dumps(line.stage)}\t{json.dumps(line.phase)}\n"
        for line in sorted(lines, key=lambda line: line.case_id)
    )
    return hashlib.sha256(text.encode()).hexdigest()


def tree_sha256(tree: str) -> str:
    """The hash a fingerprint covers for one accepted case's compact tree JSON."""
    return hashlib.sha256(tree.encode()).hexdigest()


def _verdict(*, line: CaseLine, tree: str | None) -> str:
    """The checker's verdict text: accepted with its tree hash, rejected, or the failure's name."""
    if line.verdict == ACCEPTED_VERDICT:
        return f"accepted\t{tree or ''}"
    if line.verdict == REJECTED_VERDICT:
        return "rejected\t"
    failure: str = line.verdict.removeprefix("failed:")
    return "failed:" + "".join(part.capitalize() for part in failure.split("_")) + "\t"
