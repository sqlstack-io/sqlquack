"""Reviewed inputs: hand forms, page dispositions and block reviews bound to their hashes."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.grammar_catalogue.models import HandForm
from scripts.syntax_catalogue_bigquery.models import BlockReview, HandInputs, HandPage


def strings(value: object) -> tuple[str, ...]:
    """A list of strings as a tuple; anything else as empty."""
    return tuple(str(v) for v in value) if isinstance(value, list) else ()


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
            grammars=strings(row["grammar"]),
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
    blocks: dict[tuple[str, str], BlockReview] = {}
    for row in data.get("block", []):
        review: BlockReview = BlockReview(
            source=str(row["source"]),
            block_sha256=str(row["block_sha256"]),
            status=str(row["status"]),
            reason=str(row["reason"]),
            grammar=strings(row.get("grammar", [])),
            family=str(row.get("family", "")),
        )
        blocks[(review.source, review.block_sha256)] = review
    return HandInputs(forms=forms, pages=pages, blocks=blocks, signatures={})
