"""The native module's JSON reports as typed results."""

from __future__ import annotations

import json
import os
from collections.abc import Callable, Mapping
from typing import Any

from sqlquack.api.exceptions import SqlQuackError
from sqlquack.api.models import Diagnostic, Span


def loaded(call: Callable[[], str]) -> dict[str, Any]:
    """The native call's JSON, with its errors raised as `SqlQuackError`."""
    try:
        text: str = call()
    except ValueError as error:
        raise SqlQuackError(str(error)) from None
    return json.loads(text)


def path_argument(value: str | os.PathLike[str] | None) -> str | None:
    """A path argument as the native module takes it."""
    return None if value is None else os.fspath(value)


def diagnostic(fields: Mapping[str, Any]) -> Diagnostic:
    """A diagnostic from its JSON fields."""
    span: Mapping[str, int] | None = fields["span"]
    return Diagnostic(
        code=fields["code"],
        name=fields["name"],
        alias=fields["alias"],
        severity=fields["severity"],
        message=fields["message"],
        span=None if span is None else Span(**span),
        label=fields["label"],
        notes=tuple(fields["notes"]),
        help=fields["help"],
    )
