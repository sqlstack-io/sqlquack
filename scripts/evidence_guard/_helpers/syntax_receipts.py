"""Checking one added file of syntax-coverage receipts."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.evidence_guard._helpers.git import blob
from scripts.evidence_guard.constants import SYNTAX_RECEIPT_SUFFIX, SYNTAX_REPORT_SUFFIX


def added_syntax_receipt_problems(*, repository: Path, head: str, path: str) -> list[str]:
    """Why the added receipts file `path` is not JSON Lines of objects, if it is not."""
    text: str = (blob(repository=repository, head=head, path=path) or b"").decode(
        "utf-8", errors="replace"
    )
    if path.endswith(SYNTAX_REPORT_SUFFIX):
        return [] if isinstance(_parsed(text), dict) else [f"{path}: not a JSON object"]
    if not path.endswith(SYNTAX_RECEIPT_SUFFIX):
        return [f"{path}: syntax receipts are {SYNTAX_RECEIPT_SUFFIX} files"]
    lines: list[str] = text.splitlines()
    if not lines:
        return [f"{path}: holds no receipts"]
    problems: list[str] = []
    for number, line in enumerate(lines, start=1):
        if not isinstance(_parsed(line), dict):
            problems.append(f"{path}:{number}: not a JSON object")
    return problems


def _parsed(text: str) -> object:
    """The JSON value of `text`, or None when it is not JSON."""
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return None
