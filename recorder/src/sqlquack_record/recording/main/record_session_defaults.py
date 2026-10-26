"""The account's default session parameters, read from a session that sets none."""

from __future__ import annotations

import json
from collections.abc import Callable
from pathlib import Path

from sqlquack_record.recording._helpers.results import parameters_sha256
from sqlquack_record.recording.constants import (
    RECORDER_NAME,
    RECORDER_VERSION,
    SESSION_DEFAULTS_FORMAT,
)
from sqlquack_record.recording.exceptions import RecorderError
from sqlquack_record.recording.types import SessionFactory, WarehouseSession


def record_session_defaults(
    *, out: Path, names: frozenset[str], open_session: SessionFactory, clock: Callable[[], str]
) -> int:
    """Write the `names` parameters of a session that sets none to `out`; the count."""
    session: WarehouseSession = open_session(())
    try:
        reported: dict[str, str] = session.session_parameters()
    finally:
        session.close()
    missing: list[str] = sorted(names - reported.keys())
    if missing:
        raise RecorderError(f"the session reports no value for {', '.join(missing)}")
    parameters: dict[str, str] = {name: reported[name] for name in names}
    document: dict[str, object] = {
        "format": SESSION_DEFAULTS_FORMAT,
        "recorder": f"{RECORDER_NAME} {RECORDER_VERSION}",
        "recorded_at": clock(),
        "parameters": dict(sorted(parameters.items())),
        "parameters_sha256": parameters_sha256(parameters),
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return len(parameters)
