"""Running SQLQuack's command line on each statement and reading whether it parses."""

from __future__ import annotations

import json
import os
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any

from scripts.referee_tools.constants import (
    CHECK_WORKERS,
    CONVERT_ARGUMENTS,
    DOES_NOT_PARSE,
    PARSE_FIDELITY,
)
from scripts.referee_tools.models import ParseVerdict


def parse_verdicts(
    *, binary: Path, library: Path, statements: list[str]
) -> dict[str, ParseVerdict]:
    """SQLQuack's parse verdict for each distinct statement, by statement text."""
    environment: dict[str, str] = {**os.environ, "LD_LIBRARY_PATH": str(library)}
    distinct: list[str] = list(dict.fromkeys(statements))
    with ThreadPoolExecutor(max_workers=CHECK_WORKERS) as pool:
        verdicts: list[ParseVerdict] = list(
            pool.map(
                lambda statement: _verdict(
                    binary=binary, environment=environment, statement=statement
                ),
                distinct,
            )
        )
    return dict(zip(distinct, verdicts, strict=True))


def _verdict(*, binary: Path, environment: dict[str, str], statement: str) -> ParseVerdict:
    completed: subprocess.CompletedProcess[str] = subprocess.run(
        (str(binary), *CONVERT_ARGUMENTS),
        input=statement,
        capture_output=True,
        text=True,
        env=environment,
        check=False,
    )
    try:
        report: dict[str, Any] = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return ParseVerdict(parses=False, reason=f"convert failed: {completed.stderr.strip()}")
    for refusal in report.get("refusals") or []:
        reason: str = str(refusal.get("reason", ""))
        if refusal.get("category") == PARSE_FIDELITY and reason.startswith(DOES_NOT_PARSE):
            return ParseVerdict(parses=False, reason=reason)
    return ParseVerdict(parses=True, reason="")
