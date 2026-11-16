"""One form's case results from the checker, and one re-bound review's claims."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class CaseLine:
    """What a form fingerprint covers for one case; `tree_sha256` is None unless accepted."""

    case_id: str
    sql_sha256: str
    verdict: str
    tree_sha256: str | None
    expect: str | None
    stage: str | None
    phase: str | None


@dataclass(frozen=True)
class Rebound:
    """A form a re-bound review names: its new fingerprint and the one it was approved at before."""

    review_id: str
    form_id: str
    fingerprint: str
    previous: str | None
