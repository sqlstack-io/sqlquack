"""Forms catalogued by hand, read from coverage/syntax/hand.toml."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.syntax_catalogue.constants import HAND_BASES, HAND_KINDS
from scripts.syntax_catalogue.exceptions import CatalogueError
from scripts.syntax_catalogue.models import FormGroup, HandForm


def hand_form(row: dict[str, object]) -> HandForm:
    """One validated hand form."""
    grammar: object = row["grammar"]
    form: HandForm = HandForm(
        source=str(row["source"]),
        anchor=str(row["anchor"]),
        kind=str(row["kind"]),
        label=str(row.get("label", "")),
        grammars=tuple(str(g) for g in grammar) if isinstance(grammar, list) else (),
        basis=str(row["basis"]),
        page_sha256=str(row["page_sha256"]),
    )
    if form.kind not in HAND_KINDS or form.basis not in HAND_BASES or not form.grammars:
        raise CatalogueError(f"{form.source} '{form.anchor}': bad kind, basis or grammar")
    return form


def load_hand_forms(path: Path) -> dict[str, list[HandForm]]:
    """Hand forms by source id; an absent file means none."""
    if not path.exists():
        return {}
    by_source: dict[str, list[HandForm]] = {}
    for row in tomllib.loads(path.read_text()).get("form", []):
        form: HandForm = hand_form(row)
        by_source.setdefault(form.source, []).append(form)
    return by_source


def hand_groups(*, forms: list[HandForm], title: str, page_sha256: str) -> list[FormGroup]:
    """Form groups for a page's hand forms, refusing forms read from another page version."""
    groups: list[FormGroup] = []
    for form in forms:
        if form.page_sha256 != page_sha256:
            raise CatalogueError(
                f"{form.source} '{form.anchor}': page changed since it was catalogued by hand"
            )
        heading: tuple[str, ...] = (title, *form.anchor.split(" / "))
        groups.append(FormGroup(form.kind, form.label, form.grammars, heading))
    return groups
