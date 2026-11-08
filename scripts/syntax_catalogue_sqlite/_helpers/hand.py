"""Reviewed inputs: diagram transcriptions, hand forms, overrides and page statuses."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.syntax_catalogue.models import HandForm
from scripts.syntax_catalogue_sqlite.models import HandInputs, HandPage, Override, Transcription
from scripts.syntax_catalogue_sqlite.types import Row


def strings(value: object) -> tuple[str, ...]:
    """A TOML list of strings as a tuple; anything else as an empty tuple."""
    return tuple(str(item) for item in value) if isinstance(value, list) else ()


def load_hand(path: Path) -> HandInputs:
    """hand.toml, or nothing reviewed yet when it does not exist."""
    data: dict[str, list[Row]] = tomllib.loads(path.read_text()) if path.exists() else {}
    diagrams: dict[str, Transcription] = {
        str(row["name"]): Transcription(
            name=str(row["name"]),
            svg_sha256=str(row["svg_sha256"]),
            grammar=strings(row["grammar"]),
            labels=strings(row.get("labels", [])),
            note=str(row.get("note", "")),
        )
        for row in data.get("diagram", [])
    }
    forms: dict[str, list[tuple[str, HandForm]]] = {}
    for row in data.get("form", []):
        form: HandForm = HandForm(
            source=str(row["source"]),
            anchor=str(row["anchor"]),
            kind=str(row["kind"]),
            label=str(row["label"]),
            grammars=strings(row["grammar"]),
            basis=str(row["basis"]),
            page_sha256=str(row["page_sha256"]),
        )
        forms.setdefault(form.source, []).append((str(row["family"]), form))
    overrides: dict[tuple[str, str], Override] = {
        (str(row["source"]), str(row["text"])): Override(
            source=str(row["source"]),
            text=str(row["text"]),
            grammar=strings(row["grammar"]),
            reason=str(row["reason"]),
        )
        for row in data.get("override", [])
    }
    pages: dict[str, HandPage] = {
        str(row["source"]): HandPage(
            source=str(row["source"]),
            status=str(row["status"]),
            reason=str(row["reason"]),
            page_sha256=str(row["page_sha256"]),
        )
        for row in data.get("page", [])
    }
    return HandInputs(diagrams=diagrams, forms=forms, overrides=overrides, pages=pages)
