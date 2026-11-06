"""What `sqlquack` does with a statement, as the everyday corpus test reads it."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any

from scripts.basics_corpus.constants import (
    PARSE_FAILURE_CODE,
    POLICY_FILE,
    SCHEMA_FILE,
    TRANSLATED,
)


def outcomes(*, binary: Path, repository: Path, sql: str) -> tuple[str, str]:
    """The first check diagnostic, else the first refusal or `translated`, under each policy."""
    checked: dict[str, Any] = json.loads(
        _run(command=[str(binary), "check", "-", "--json"], sql=sql)
    )
    if checked["diagnostics"]:
        code: str = checked["diagnostics"][0]["code"]
        found: str = f"{'parse' if code == PARSE_FAILURE_CODE else 'check'} {code}"
        return found, found
    schema: list[str] = ["--schema", str(repository / SCHEMA_FILE)]
    strict: str = _converted(binary=binary, sql=sql, flags=[*schema, "--builtin-policy"])
    policy: list[str] = ["--policy", str(repository / POLICY_FILE), "--utc-session"]
    return strict, _converted(binary=binary, sql=sql, flags=[*schema, *policy])


def _converted(*, binary: Path, sql: str, flags: list[str]) -> str:
    report: dict[str, Any] = json.loads(
        _run(command=[str(binary), "convert", "-", "--json", *flags], sql=sql)
    )
    if not report["refusals"]:
        return TRANSLATED
    first: dict[str, Any] = report["refusals"][0]
    return f"refused {first.get('code', '')} {first['construct']}"


def _run(*, command: list[str], sql: str) -> str:
    return subprocess.run(command, input=sql, capture_output=True, text=True, check=False).stdout
