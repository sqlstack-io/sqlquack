"""The sizes format, the growth allowed per case, the summary's thresholds and allowance fields."""

from __future__ import annotations

SIZES_FORMAT: str = "sqlquack-sql-sizes/1"
GROWTH_BYTES: int = 64
GROWTH_SHARE: float = 0.10
LARGE_BYTES: tuple[int, ...] = (1024, 4096)
REQUIRED_TEXT: tuple[str, ...] = ("reason", "ruling")
