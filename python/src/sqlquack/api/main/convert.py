"""`convert`: what `sqlquack convert` reports, with one option per command-line flag."""

from __future__ import annotations

import json
from typing import Any

from sqlquack._native import convert_json
from sqlquack.api._helpers.reports import diagnostic, loaded, path_argument
from sqlquack.api.constants import DEFAULT_PATH
from sqlquack.api.models import Conversion, ConvertOptions


def convert(
    *, sql: str, options: ConvertOptions | None = None, path: str = DEFAULT_PATH
) -> Conversion:
    """Convert `sql` or refuse it as `sqlquack convert` does; exit-2 failures raise."""
    chosen: ConvertOptions = options or ConvertOptions()
    request: dict[str, Any] = {
        "project": path_argument(chosen.project),
        "schema": None if chosen.schema is None else dict(chosen.schema),
        "config": path_argument(chosen.config),
        "no_config": chosen.no_config,
        "session": dict(chosen.session),
        "strict": chosen.strict,
        "python_runtime": chosen.python_runtime,
        "overrides": path_argument(chosen.overrides),
        "verified_overrides": dict(chosen.verified_overrides),
        "compact": chosen.compact,
    }
    result: dict[str, Any] = loaded(lambda: convert_json(sql, json.dumps(request), path))
    report: dict[str, Any] = result["report"]
    return Conversion(
        converted=report["converted"],
        sql=report["sql"],
        macros=tuple(report["macros"]),
        profile=report["profile"],
        diagnostics=tuple(diagnostic(refusal) for refusal in report["refusals"]),
        divergences={name: tuple(uses) for name, uses in report["divergences"].items()},
        assumptions=tuple(report["assumptions"]),
        divergence_failures=dict(report["divergence_failures"]),
        notes=tuple(report["notes"]),
        rendered=result["rendered"],
        report=report,
    )
