"""What marks a re-bound review, the call nodes it may differ in, and the names it reads."""

from __future__ import annotations

import re

REBOUND_NOTE: str = "re-bound: set_quantifier rename, diff-checked"
CALL_NODES: tuple[str, ...] = ("function", "aggregate_function")
OLD_FIELD: str = "distinct"
NEW_FIELD: str = "set_quantifier"
QUANTIFIER_OF: dict[bool, str] = {True: "distinct", False: "none"}
EXPLICIT_ALL: str = "all"
REJECTED: str = "REJECTED"
ACCEPTED_VERDICT: str = "accepted"
REJECTED_VERDICT: str = "rejected"
FAILED_VERDICT: str = "failed"
APPROVED: str = "approved"
WRITTEN_ALL: re.Pattern[str] = re.compile(r"\(\s*ALL\b", re.IGNORECASE)
