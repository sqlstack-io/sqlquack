"""Type aliases the generators share."""

from __future__ import annotations

from decimal import Decimal
from enum import StrEnum

type Pair = tuple[Decimal | None, Decimal | None]
type Row = tuple[str, ...]


class Operator(StrEnum):
    """An arithmetic operator."""

    ADD = "+"
    SUBTRACT = "-"
    MULTIPLY = "*"
    DIVIDE = "/"


class Target(StrEnum):
    """What a scalar projection of a path casts to, for deciding which documents convert."""

    NUMBER = "number"
    BOOLEAN = "boolean"
