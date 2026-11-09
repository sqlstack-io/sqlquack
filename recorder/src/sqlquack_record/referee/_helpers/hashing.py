"""The hashes naming referee environments and cases."""

from __future__ import annotations

import hashlib
import json
from collections.abc import Mapping, Sequence

from sqlquack_record.recording.types import SessionValue
from sqlquack_record.referee.constants import REFEREE_FORMAT


def environment_sha256(
    *, session_parameters: Mapping[str, SessionValue], setup_sql: Sequence[str]
) -> str:
    """SHA-256 of an environment's canonical JSON."""
    return _canonical_sha256(
        {
            "format": REFEREE_FORMAT,
            "session_parameters": dict(session_parameters),
            "setup_sql": list(setup_sql),
        }
    )


def case_sha256(*, source_dialect: str, environment: str, statement: str, syntax_only: bool) -> str:
    """SHA-256 of a case's canonical JSON, which names its environment by hash."""
    return _canonical_sha256(
        {
            "format": REFEREE_FORMAT,
            "source_dialect": source_dialect,
            "environment_sha256": environment,
            "statement": statement,
            "syntax_only": syntax_only,
        }
    )


def _canonical_sha256(document: object) -> str:
    text: str = json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(text.encode("utf-8")).hexdigest()
