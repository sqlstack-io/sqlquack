"""`check`: what `sqlquack check` reports."""

from __future__ import annotations

from typing import Any

from sqlquack._native import check_json
from sqlquack.api._helpers.reports import diagnostic, loaded
from sqlquack.api.constants import DEFAULT_PATH
from sqlquack.api.models import CheckResult


def check(*, sql: str, path: str = DEFAULT_PATH) -> CheckResult:
    """Parse `sql` and find what Snowflake rejects, naming it `path`; no SQL raises."""
    result: dict[str, Any] = loaded(lambda: check_json(sql, path))
    report: dict[str, Any] = result["report"]
    return CheckResult(
        ok=report["ok"],
        diagnostics=tuple(diagnostic(found) for found in report["diagnostics"]),
        rendered=result["rendered"],
    )
