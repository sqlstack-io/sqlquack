"""A project function's signature, as a project folder's function file declares it."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Signature:
    """The language, parameters and return of one compiled function."""

    language: str
    arguments: tuple[tuple[str, str], ...]
    returns: str | tuple[tuple[str, str], ...]
