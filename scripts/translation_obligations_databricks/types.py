"""Type aliases of the Databricks translation obligation builder."""

from __future__ import annotations

from typing import Any

type Case = dict[str, Any]
type Entry = tuple[str, str, Case, tuple[str, str]]
