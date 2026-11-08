"""The growth allowances that apply against one base baseline."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass(frozen=True)
class Allowances:
    """Each allowed case's largest size, the extra total growth allowed, and unused entries."""

    cases: dict[str, int] = field(default_factory=dict)
    total: int = 0
    unused: tuple[str, ...] = ()
