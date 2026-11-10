"""Test types for the workspace lockfile guard."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class LockTestCase:
    description: str
    manifest_after_lock: str
    expected_return_code: int
    expected_fragment: str
