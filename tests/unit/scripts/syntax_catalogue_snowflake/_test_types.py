"""Test types for the Snowflake catalogue build's page refresh and id keeping."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class MintedIdTestCase:
    description: str
    old: str
    natural: str
    expected_minted: bool


@dataclass(frozen=True)
class RefreshTestCase:
    description: str
    grammar: str
    expected_ids: tuple[str, ...]
    expected_page_sha256: str
