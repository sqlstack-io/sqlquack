"""Reviewed inputs: hand forms, dispositions of pages without syntax, and signature exceptions."""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import cast

from scripts.syntax_catalogue.models import HandForm
from scripts.syntax_catalogue_duckdb.models import HandInputs, HandPage


def load_hand(path: Path) -> HandInputs:
    """hand.toml, or nothing reviewed yet when it does not exist."""
    data: dict[str, list[dict[str, object]]] = (
        tomllib.loads(path.read_text()) if path.exists() else {}
    )
    forms: dict[str, list[tuple[str, HandForm]]] = {}
    for row in data.get("form", []):
        form: HandForm = HandForm(
            source=str(row["source"]),
            anchor=str(row["anchor"]),
            kind=str(row["kind"]),
            label=str(row["label"]),
            grammars=tuple(str(grammar) for grammar in cast("list[object]", row["grammar"])),
            basis=str(row["basis"]),
            page_sha256=str(row["page_sha256"]),
        )
        forms.setdefault(form.source, []).append((str(row["family"]), form))
    pages: dict[str, HandPage] = {
        str(row["source"]): HandPage(
            source=str(row["source"]),
            status=str(row["status"]),
            reason=str(row["reason"]),
            page_sha256=str(row["page_sha256"]),
        )
        for row in data.get("page", [])
    }
    signatures: dict[tuple[str, str], str] = {
        (str(row["source"]), str(row["signature"])): str(row["reason"])
        for row in data.get("signature", [])
    }
    return HandInputs(forms=forms, pages=pages, signatures=signatures)
