"""Refuse each re-bound review whose forms changed by more than its class allows."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

from scripts.rebound_reviews._helpers.inputs import cases, dump, form_cases, rebound
from scripts.rebound_reviews._helpers.refusals import refusal, stage_refusal
from scripts.rebound_reviews.constants import CHECK_EPILOG, STAGE_CLASS
from scripts.rebound_reviews.models import CaseLine, Rebound


def run_rebound_check(argv: list[str] | None = None) -> int:
    """Fail unless each re-bound form's reviewed trees, by fingerprint, map to its current ones."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(
        description=__doc__, epilog=CHECK_EPILOG
    )
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
            stage_refusal(claim=claim, forms=forms, recorded=recorded, before=before)
            if claim.diff_class == STAGE_CLASS
            else refusal(claim=claim, forms=forms, recorded=recorded, old=old, new=new)
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
