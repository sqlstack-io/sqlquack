"""Reviewed inputs from hand.toml: hand forms, synopsis block dispositions and exclusions."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.syntax_catalogue.exceptions import CatalogueError
from scripts.syntax_catalogue_postgres.constants import (
    BLOCK_KEYS,
    BLOCK_KINDS,
    FOLD_KEYS,
    FORM_KEYS,
    HAND_BASES,
    HAND_KINDS,
    PAGE_KEYS,
    SECTION_KEYS,
)
from scripts.syntax_catalogue_postgres.models import Disposition, HandForm, HandInputs


def check_keys(*, table: str, row: dict[str, object], allowed: frozenset[str]) -> None:
    """Refuse a row with an unknown field."""
    unknown: set[str] = set(row) - allowed
    if unknown:
        raise CatalogueError(
            f"hand.toml [[{table}]] {row.get('source')}: unknown {sorted(unknown)}"
        )


def strings(value: object) -> tuple[str, ...]:
    """A list of strings as a tuple; anything else as empty."""
    return tuple(str(v) for v in value) if isinstance(value, list) else ()


def load_hand(path: Path) -> HandInputs:
    """Read and validate hand.toml; an absent file means no hand input."""
    inputs: HandInputs = HandInputs()
    if not path.exists():
        return inputs
    data: dict[str, list[dict[str, object]]] = tomllib.loads(path.read_text())
    for row in data.get("form", []):
        check_keys(table="form", row=row, allowed=FORM_KEYS)
        form: HandForm = HandForm(
            source=str(row["source"]),
            anchor=str(row["anchor"]),
            kind=str(row["kind"]),
            label=str(row.get("label", "")),
            family=str(row.get("family", "")),
            grammars=strings(row["grammar"]),
            basis=str(row["basis"]),
            page_sha256=str(row["page_sha256"]),
        )
        if form.kind not in HAND_KINDS or form.basis not in HAND_BASES or not form.grammars:
            raise CatalogueError(f"{form.source} '{form.anchor}': bad kind, basis or grammar")
        inputs.forms.setdefault(form.source, []).append(form)
    for row in data.get("block", []):
        check_keys(table="block", row=row, allowed=BLOCK_KEYS)
        block: Disposition = Disposition(
            source=str(row["source"]),
            block_sha256=str(row["block_sha256"]),
            disposition=str(row["disposition"]),
            kind=str(row.get("kind", "")),
            family=str(row.get("family", "")),
            label=str(row.get("label", "")),
            labels=strings(row.get("labels")),
            whole=bool(row.get("whole", False)),
            names=strings(row.get("names")),
            listed=bool(row.get("listed", False)),
            forms=strings(row.get("forms")),
            reason=str(row.get("reason", "")),
        )
        if block.disposition not in BLOCK_KINDS:
            raise CatalogueError(f"{block.source} {block.block_sha256}: bad disposition")
        key: tuple[str, str] = (block.source, block.block_sha256)
        if key in inputs.blocks:
            raise CatalogueError(f"{block.source} {block.block_sha256}: dispositioned twice")
        inputs.blocks[key] = block
    for row in data.get("signatures", []):
        check_keys(table="signatures", row=row, allowed=SECTION_KEYS)
        inputs.sections[(str(row["source"]), str(row["anchor"]))] = str(row["reason"])
    for row in data.get("page", []):
        check_keys(table="page", row=row, allowed=PAGE_KEYS)
        inputs.pages[str(row["source"])] = str(row["reason"])
    for row in data.get("fold", []):
        check_keys(table="fold", row=row, allowed=FOLD_KEYS)
        inputs.folds[str(row["source"])] = (strings(row.get("forms")), str(row["reason"]))
    return inputs
