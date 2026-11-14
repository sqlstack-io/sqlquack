"""Models of syntax_catalogue_snowflake."""

from __future__ import annotations

from dataclasses import dataclass

from scripts.grammar_catalogue.models import Existing, HandForm


@dataclass(frozen=True)
class BuildContext:
    """What one page of a build reads: command leading words, its hand forms, earlier ids."""

    lead: set[str]
    hand: list[HandForm]
    existing: Existing
