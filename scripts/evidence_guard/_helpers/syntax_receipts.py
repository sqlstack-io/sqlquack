"""Checking one added file of syntax-coverage receipts."""

from __future__ import annotations

import json
import re
from pathlib import Path

from scripts.evidence_guard._helpers.git import blob
from scripts.evidence_guard.constants import (
    SHA256_PATTERN,
    SYNTAX_RECEIPT_SUFFIX,
    SYNTAX_REPORT_SUFFIX,
    SYNTAX_WITHDRAWN_KEY,
    SYNTAX_WITHDRAWN_REASON_KEY,
    SYNTAX_WITHDRAWN_RECEIPT_KEY,
)


def added_syntax_receipt_problems(*, repository: Path, head: str, path: str) -> list[str]:
    """Why the added receipts file or run report `path` is malformed, if it is."""
    text: str = (blob(repository=repository, head=head, path=path) or b"").decode(
        "utf-8", errors="replace"
    )
    if path.endswith(SYNTAX_REPORT_SUFFIX):
        report: object = _parsed(text)
        if not isinstance(report, dict):
            return [f"{path}: not a JSON object"]
        return _withdrawal_problems(path=path, entries=report.get(SYNTAX_WITHDRAWN_KEY, []))
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


def _withdrawal_problems(*, path: str, entries: object) -> list[str]:
    """Why a report's withdrawals do not each name a receipt line's hash and a reason."""
    if not isinstance(entries, list):
        return [f"{path}: {SYNTAX_WITHDRAWN_KEY} is not a list"]
    problems: list[str] = []
    for number, entry in enumerate(entries, start=1):
        fields: dict[str, object] = entry if isinstance(entry, dict) else {}
        receipt: object = fields.get(SYNTAX_WITHDRAWN_RECEIPT_KEY)
        reason: object = fields.get(SYNTAX_WITHDRAWN_REASON_KEY)
        if not isinstance(receipt, str) or not re.fullmatch(SHA256_PATTERN, receipt):
            problems.append(f"{path}: withdrawal {number} names no {SYNTAX_WITHDRAWN_RECEIPT_KEY}")
        if not isinstance(reason, str) or not reason.strip():
            problems.append(f"{path}: withdrawal {number} gives no {SYNTAX_WITHDRAWN_REASON_KEY}")
    return problems


def _parsed(text: str) -> object:
    """The JSON value of `text`, or None when it is not JSON."""
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return None
