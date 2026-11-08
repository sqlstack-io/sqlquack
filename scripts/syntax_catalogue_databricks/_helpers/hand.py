"""Reviewed inputs of the catalogue: hand forms, page dispositions and block readings."""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import cast

from scripts.syntax_catalogue_databricks.constants import (
    HAND_BASES,
    HAND_FORM_KEYS,
    HAND_KINDS,
    HAND_PAGE_KEYS,
    HAND_READING_KEYS,
    PAGE_DISPOSITIONS,
)
from scripts.syntax_catalogue_databricks.exceptions import DatabricksCatalogueError
from scripts.syntax_catalogue_databricks.models import Hand, HandForm, HandPage, HandReading
from scripts.syntax_catalogue_databricks.types import Row


def _checked(*, table: str, row: Row, keys: frozenset[str]) -> Row:
    unknown: set[str] = set(row) - keys
    if unknown:
        raise DatabricksCatalogueError(f"hand.toml [[{table}]]: unknown keys {sorted(unknown)}")
    return row


def _lines(value: object) -> tuple[str, ...]:
    """A hand row's list of grammar lines."""
    return tuple(str(line) for line in cast(list[object], value))


def load_hand(path: Path) -> Hand:
    """hand.toml's [[form]], [[page]] and [[reading]] rows; an unknown key stops the build."""
    data: dict[str, list[Row]] = tomllib.loads(path.read_text()) if path.exists() else {}
    forms: list[HandForm] = []
    for row in data.get("form", []):
        _checked(table="form", row=row, keys=HAND_FORM_KEYS)
        if row["kind"] not in HAND_KINDS or row["basis"] not in HAND_BASES:
            raise DatabricksCatalogueError(f"hand form {row['label']}: kind or basis unknown")
        forms.append(
            HandForm(
                page=str(row["page"]),
                anchor=str(row["anchor"]),
                kind=str(row["kind"]),
                label=str(row["label"]),
                grammars=_lines(row["grammar"]),
                basis=str(row["basis"]),
                article_sha256=str(row["article_sha256"]),
            )
        )
    pages: list[HandPage] = []
    for row in data.get("page", []):
        _checked(table="page", row=row, keys=HAND_PAGE_KEYS)
        if row["status"] not in PAGE_DISPOSITIONS:
            raise DatabricksCatalogueError(f"hand page {row['page']}: status {row['status']}")
        pages.append(
            HandPage(
                page=str(row["page"]),
                status=str(row["status"]),
                reason=str(row["reason"]),
                article_sha256=str(row["article_sha256"]),
            )
        )
    readings: list[HandReading] = []
    for row in data.get("reading", []):
        _checked(table="reading", row=row, keys=HAND_READING_KEYS)
        readings.append(
            HandReading(
                page=str(row["page"]),
                written=str(row["written"]),
                read=_lines(row["read"]),
                reason=str(row["reason"]),
            )
        )
    return Hand(forms=forms, pages=pages, readings=readings)
